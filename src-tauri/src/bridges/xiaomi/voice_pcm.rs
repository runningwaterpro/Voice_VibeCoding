//! 对齐 Python `UdpPcmOutput`：16k→48k 后 UDP 送到独立 audio_router 进程
//!
//! - `clear()` → `CLEAR`：语音键按下，router 开 CABLE 输出流
//! - `end_session()` / `stop()` → `END`：会话结束，router debounce 后关流

use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::audio::pcm_router::DEFAULT_PCM_PORT;

struct Client {
    sock: UdpSocket,
    peer: SocketAddr,
    prev: i16,
    have_prev: bool,
    sent: AtomicU64,
    dropped: AtomicU64,
}

static CLIENT: Mutex<Option<Client>> = Mutex::new(None);
/// 热路径快速判断，避免每帧进 ensure_started / 抢锁探测
static READY: AtomicBool = AtomicBool::new(false);
static FIRST_PACKET: AtomicBool = AtomicBool::new(false);
static WARMING: AtomicBool = AtomicBool::new(false);
static WARMUP_GENERATION: AtomicU64 = AtomicU64::new(0);
static START_LOCK: Mutex<()> = Mutex::new(());

fn pcm_port() -> u16 {
    std::env::var("REMOTE_BRIDGE_PCM_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PCM_PORT)
}

fn peer_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], pcm_port()))
}

/// PING 重试间隔（冷启动首按尽量快）
pub const PING_RETRY_INTERVAL_MS: u64 = 15;
pub const PING_DEADLINE_SECS: u64 = 4;

pub fn ping_retry_interval_ms() -> u64 {
    PING_RETRY_INTERVAL_MS
}

pub fn ping_deadline_secs() -> u64 {
    PING_DEADLINE_SECS
}

/// 语音键按下：只短暂等待现成的 warmup，避免按住物理键时阻塞四秒。
pub fn ensure_pcm_ready_on_press() -> Result<(), String> {
    if READY.load(Ordering::Acquire) {
        return Ok(());
    }
    warmup_async();
    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline {
        if READY.load(Ordering::Acquire) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err("PCM route is still warming; press again after it is ready".into())
}

/// 等待 router PONG（对齐 Python 最多 ~4s）
pub fn ensure_started() -> Result<(), String> {
    ensure_started_for(WARMUP_GENERATION.load(Ordering::Acquire))
}

fn ensure_started_for(generation: u64) -> Result<(), String> {
    if READY.load(Ordering::Acquire) {
        return Ok(());
    }
    {
        let g = CLIENT.lock();
        if g.is_some() {
            READY.store(true, Ordering::Release);
            return Ok(());
        }
    }
    if generation != WARMUP_GENERATION.load(Ordering::Acquire) {
        return Err("PCM warmup cancelled".into());
    }
    let _start = START_LOCK.lock();
    if READY.load(Ordering::Acquire) {
        return Ok(());
    }
    if CLIENT.lock().is_some() {
        READY.store(true, Ordering::Release);
        return Ok(());
    }
    if generation != WARMUP_GENERATION.load(Ordering::Acquire) {
        return Err("PCM warmup cancelled".into());
    }
    let peer = peer_addr();
    let sock = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    sock.set_read_timeout(Some(Duration::from_millis(150)))
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(PING_DEADLINE_SECS);
    let mut ok = false;
    while Instant::now() < deadline {
        if generation != WARMUP_GENERATION.load(Ordering::Acquire) {
            return Err("PCM warmup cancelled".into());
        }
        let _ = sock.send_to(b"PING", peer);
        let mut buf = [0u8; 64];
        if let Ok((n, _)) = sock.recv_from(&mut buf) {
            if &buf[..n] == b"PONG" {
                ok = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(PING_RETRY_INTERVAL_MS));
    }
    if !ok {
        return Err(format!("audio router not ready at {peer}"));
    }
    if generation != WARMUP_GENERATION.load(Ordering::Acquire) {
        return Err("PCM warmup cancelled".into());
    }
    *CLIENT.lock() = Some(Client {
        sock,
        peer,
        prev: 0,
        have_prev: false,
        sent: AtomicU64::new(0),
        dropped: AtomicU64::new(0),
    });
    READY.store(true, Ordering::Release);
    log::info!("XIAOMI VOICE PCM UDP ready peer={peer}");
    Ok(())
}

/// 后台预热：应用启动 / 连上遥控后尽早 PING，避免首句说话才建连
pub fn warmup_async() {
    if READY.load(Ordering::Acquire) || WARMING.swap(true, Ordering::AcqRel) {
        return;
    }
    let generation = WARMUP_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    std::thread::Builder::new()
        .name("xiaomi-pcm-warmup".into())
        .spawn(move || {
            for attempt in 1..=8 {
                match ensure_started_for(generation) {
                    Ok(()) => {
                        log::info!("XIAOMI VOICE PCM warmup ok attempt={attempt}");
                        if generation == WARMUP_GENERATION.load(Ordering::Acquire) {
                            WARMING.store(false, Ordering::Release);
                        }
                        return;
                    }
                    Err(e) => {
                        if generation != WARMUP_GENERATION.load(Ordering::Acquire) {
                            return;
                        }
                        log::debug!("XIAOMI VOICE PCM warmup attempt={attempt}: {e}");
                        std::thread::sleep(Duration::from_millis(250));
                    }
                }
            }
            if generation == WARMUP_GENERATION.load(Ordering::Acquire) {
                WARMING.store(false, Ordering::Release);
            }
            log::warn!("XIAOMI VOICE PCM warmup gave up; will retry on first push");
        })
        .map_err(|error| {
            if generation == WARMUP_GENERATION.load(Ordering::Acquire) {
                WARMING.store(false, Ordering::Release);
            }
            log::warn!("PCM warmup thread start failed: {error}");
        })
        .ok();
}

pub fn is_ready() -> bool {
    READY.load(Ordering::Acquire)
}

pub fn first_packet_observed() -> bool {
    FIRST_PACKET.load(Ordering::Acquire)
}

pub fn clear() -> Result<(), String> {
    let mut client = CLIENT.lock();
    let Some(c) = client.as_mut() else {
        return Err("PCM client is not ready".into());
    };
    c.sock
        .send_to(b"CLEAR", c.peer)
        .map_err(|e| e.to_string())?;
    c.have_prev = false;
    FIRST_PACKET.store(false, Ordering::Release);
    Ok(())
}

pub fn end_session() -> Result<(), String> {
    let client = CLIENT.lock();
    let Some(c) = client.as_ref() else {
        // Ending an already-clean session is idempotent.
        return Ok(());
    };
    c.sock
        .send_to(b"END", c.peer)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn push_16k(samples: &[i16]) -> Result<(), String> {
    if samples.is_empty() {
        return Err("empty PCM frame".into());
    }
    if !READY.load(Ordering::Acquire) {
        ensure_started()?;
    }
    let mut guard = CLIENT.lock();
    let Some(c) = guard.as_mut() else {
        READY.store(false, Ordering::Release);
        return Err("PCM client disappeared".into());
    };
    let mut previous = if c.have_prev { c.prev } else { samples[0] };
    let mut out = Vec::with_capacity(samples.len() * 3 * 2);
    for &current in samples {
        let delta = current as i32 - previous as i32;
        for s in [
            (previous as i32 + delta / 3) as i16,
            (previous as i32 + delta * 2 / 3) as i16,
            current,
        ] {
            out.extend_from_slice(&s.to_le_bytes());
        }
        previous = current;
    }
    c.prev = samples[samples.len() - 1];
    c.have_prev = true;
    let peer = c.peer;
    let udp_ok = match c.sock.send_to(&out, peer) {
        Ok(_) => {
            c.sent.fetch_add(1, Ordering::Relaxed);
            FIRST_PACKET.store(true, Ordering::Release);
            true
        }
        Err(_) => {
            c.dropped.fetch_add(1, Ordering::Relaxed);
            false
        }
    };
    drop(guard);
    // 增益后送声电平（仅 UDP 成功）
    crate::bridges::xiaomi::voice_meter::on_output_pcm(samples, udp_ok);
    if udp_ok {
        Ok(())
    } else {
        Err("PCM UDP send failed".into())
    }
}

pub fn stop() {
    let _start = START_LOCK.lock();
    WARMUP_GENERATION.fetch_add(1, Ordering::AcqRel);
    WARMING.store(false, Ordering::Release);
    READY.store(false, Ordering::Release);
    FIRST_PACKET.store(false, Ordering::Release);
    if let Some(c) = CLIENT.lock().take() {
        // END：让 router 关流，勿用 CLEAR（CLEAR = 会话开始开流）
        let _ = c.sock.send_to(b"END", c.peer);
    }
    crate::bridges::xiaomi::voice_meter::set_session(false);
}

pub fn stats() -> (u64, u64) {
    match CLIENT.lock().as_ref() {
        Some(c) => (
            c.sent.load(Ordering::Relaxed),
            c.dropped.load(Ordering::Relaxed),
        ),
        None => (0, 0),
    }
}
