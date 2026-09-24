//! 小米语音环境：检测 VB-CABLE，并用内嵌官方驱动包修复
//!
//! **安装**：解压内嵌 `VBCABLE_Driver_Pack45.zip`，启动官方 `VBCABLE_Setup_x64.exe`（有界面）。
//! 不再使用 SetupAPI 静默注册根设备（易双实例、Failed Start）——对齐上游 mwlt。
//! 官方安装完成后只验证端点；不修改 Windows 默认麦克风或隐私设置。
//!
//! **探测策略（长期最优）**：
//! - 优先读 MMDevices **注册表**（与 configure 脚本一致），避免 cpal/WASAPI 枚举打爆 audiodg
//! - 仅当注册表 **读失败** 时才 cpal 兜底一次；「没有 CABLE」不算失败
//! - 启动实探一次；**已就绪则停探**；**未就绪**按间隔重试（默认 60s）
//! - 「检测/修复」走 `voice_env_status_fresh` / `invalidate` 强制重探
//! - `REMOTE_BRIDGE_CABLE_PROBE_TTL_MS`：未就绪重试间隔；`0` = 未就绪也不自动重试

use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::copy;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

pub const DRIVER_ZIP_NAME: &str = "VBCABLE_Driver_Pack45.zip";
pub const DRIVER_ZIP_SHA256: &str =
    "b950e39f01af1d04ea623c8f6d8eb9b6ea5c477c637295fabf20631c85116bfb";
pub const CONFIGURE_SCRIPT_NAME: &str = "configure-xiaomi-audio.ps1";
pub const DOWNLOAD_PAGE_URL: &str = "https://vb-audio.com/Cable/";
pub const DOWNLOAD_ZIP_URL: &str =
    "https://download.vb-audio.com/Download_CABLE/VBCABLE_Driver_Pack45.zip";

/// The product is ready only when both sides of the virtual cable are present.
pub fn cable_endpoints_ready(cable_input: bool, cable_output: bool) -> bool {
    cable_input && cable_output
}

/// An installer exit code never substitutes for observing both endpoints.
pub fn installer_observed_ready(
    exit_code: Option<i32>,
    cable_input: bool,
    cable_output: bool,
) -> bool {
    let _ = exit_code;
    cable_endpoints_ready(cable_input, cable_output)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceEnvStatus {
    pub ready: bool,
    pub cable_input: bool,
    pub cable_output: bool,
    pub embedded_available: bool,
    pub embedded_zip_path: Option<String>,
    pub download_page_url: String,
    pub download_zip_url: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceEnvActionResult {
    pub ok: bool,
    pub ready: bool,
    pub needs_choice: bool,
    pub needs_reboot: bool,
    pub message: String,
    pub report_path: Option<String>,
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    copy(&mut reader, &mut hasher).map_err(|e| format!("hash {}: {e}", path.display()))?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn asset_candidates(file_name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("REMOTE_BRIDGE_XIAOMI_VB_CABLE_ZIP") {
        if file_name == DRIVER_ZIP_NAME {
            out.push(PathBuf::from(p));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("assets").join("xiaomi").join(file_name));
            out.push(
                dir.join("resources")
                    .join("assets")
                    .join("xiaomi")
                    .join(file_name),
            );
            out.push(
                dir.join("_up_")
                    .join("resources")
                    .join("assets")
                    .join("xiaomi")
                    .join(file_name),
            );
            if let Some(parent) = dir.parent() {
                out.push(
                    parent
                        .join("resources")
                        .join("assets")
                        .join("xiaomi")
                        .join(file_name),
                );
            }
        }
    }
    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        out.push(
            PathBuf::from(manifest)
                .join("assets")
                .join("xiaomi")
                .join(file_name),
        );
    }
    out
}

pub fn find_driver_zip() -> Option<PathBuf> {
    for path in asset_candidates(DRIVER_ZIP_NAME) {
        if !path.is_file() {
            continue;
        }
        match sha256_file(&path) {
            Ok(hash) if hash.eq_ignore_ascii_case(DRIVER_ZIP_SHA256) => return Some(path),
            Ok(hash) => log::warn!(
                "VB-CABLE zip hash mismatch path={} got={hash}",
                path.display()
            ),
            Err(e) => log::warn!("VB-CABLE zip unreadable: {e}"),
        }
    }
    None
}

pub fn find_configure_script() -> Option<PathBuf> {
    asset_candidates(CONFIGURE_SCRIPT_NAME)
        .into_iter()
        .find(|p| p.is_file())
}

/// 官方安装器文件名（64 位优先 x64）
pub fn official_setup_exe_name() -> &'static str {
    if cfg!(target_pointer_width = "64") {
        "VBCABLE_Setup_x64.exe"
    } else {
        "VBCABLE_Setup.exe"
    }
}

fn official_setup_fallback_name() -> &'static str {
    if cfg!(target_pointer_width = "64") {
        "VBCABLE_Setup.exe"
    } else {
        "VBCABLE_Setup_x64.exe"
    }
}

const STAGE_DIR_NAME: &str = "vb-cable-official-stage";

fn local_app_data_stage_root() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("Voice VibeCoding").join(STAGE_DIR_NAME)
}

fn path_parent_writable(parent: &Path) -> bool {
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    let probe = parent.join(format!(".vb-cable-wprobe-{}", std::process::id()));
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn stage_root() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if path_parent_writable(dir) {
                return dir.join(STAGE_DIR_NAME);
            }
            let resources = dir.join("resources");
            if path_parent_writable(&resources) {
                return resources.join(STAGE_DIR_NAME);
            }
            if let Some(parent) = dir.parent() {
                if path_parent_writable(parent) {
                    return parent.join(STAGE_DIR_NAME);
                }
                let parent_resources = parent.join("resources");
                if path_parent_writable(&parent_resources) {
                    return parent_resources.join(STAGE_DIR_NAME);
                }
            }
        }
    }
    local_app_data_stage_root()
}

fn find_file_named(root: &Path, name: &str) -> Option<PathBuf> {
    let direct = root.join(name);
    if direct.is_file() {
        return Some(direct);
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() {
                stack.push(p);
            } else if p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
            {
                return Some(p);
            }
        }
    }
    None
}

/// 解压内嵌 zip 到本地 staging，返回官方 Setup.exe 路径
pub fn stage_official_setup(zip: &Path) -> Result<PathBuf, String> {
    let hash = sha256_file(zip)?;
    if !hash.eq_ignore_ascii_case(DRIVER_ZIP_SHA256) {
        return Err(format!(
            "内嵌驱动包校验失败（hash={hash}），请改用官网包或重装本软件"
        ));
    }
    let stage = stage_root();
    log::info!(
        "XIAOMI VOICE ENV: staging official setup under {}",
        stage.display()
    );
    let marker = stage.join(".zip.sha256");
    let preferred = official_setup_exe_name();
    let reuse = marker
        .is_file()
        .then(|| std::fs::read_to_string(&marker).ok())
        .flatten()
        .is_some_and(|h| h.trim().eq_ignore_ascii_case(&hash))
        && find_file_named(&stage, preferred).is_some();
    if !reuse {
        if stage.exists() {
            let _ = std::fs::remove_dir_all(&stage);
        }
        std::fs::create_dir_all(&stage).map_err(|e| format!("创建解压目录失败: {e}"))?;
        let status = Command::new("tar")
            .args([
                "-xf",
                &zip.display().to_string(),
                "-C",
                &stage.display().to_string(),
            ])
            .status()
            .map_err(|e| format!("解压驱动包失败（tar）: {e}"))?;
        if !status.success() {
            let ps = format!(
                "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
                zip.display().to_string().replace('\'', "''"),
                stage.display().to_string().replace('\'', "''")
            );
            let st = Command::new("powershell.exe")
                .args(["-NoProfile", "-Command", &ps])
                .status()
                .map_err(|e| format!("解压驱动包失败（Expand-Archive）: {e}"))?;
            if !st.success() {
                return Err("解压 VB-CABLE 驱动包失败".into());
            }
        }
        std::fs::write(&marker, hash).map_err(|e| format!("写解压标记失败: {e}"))?;
    }
    find_file_named(&stage, preferred)
        .or_else(|| find_file_named(&stage, official_setup_fallback_name()))
        .ok_or_else(|| format!("解压后未找到官方安装程序（期望 {preferred}）"))
}

/// 无黑框启动 GUI 安装器并等待退出，返回进程退出码。
#[cfg(target_os = "windows")]
fn launch_gui_exe_and_wait(exe: &Path) -> Result<Option<i32>, String> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, GetLastError, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let cwd = exe
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(stage_root);
    let file_h = HSTRING::from(exe.as_os_str());
    let cwd_h = HSTRING::from(cwd.as_os_str());

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpFile: PCWSTR(file_h.as_ptr()),
        lpDirectory: PCWSTR(cwd_h.as_ptr()),
        nShow: SW_SHOWNORMAL.0 as i32,
        ..Default::default()
    };

    let ok = unsafe { ShellExecuteExW(&mut info).is_ok() };
    if !ok {
        let code = unsafe { GetLastError() }.0;
        return Err(format!("启动官方安装程序失败 GetLastError={code}"));
    }
    if info.hProcess.is_invalid() {
        return Ok(Some(0));
    }
    unsafe {
        let wait = WaitForSingleObject(info.hProcess, INFINITE);
        if wait != WAIT_OBJECT_0 {
            let _ = CloseHandle(info.hProcess);
            return Err("等待官方安装程序结束失败".into());
        }
        let mut code: u32 = 0;
        let _ = GetExitCodeProcess(info.hProcess, &mut code);
        let _ = CloseHandle(info.hProcess);
        Ok(Some(code as i32))
    }
}

#[cfg(not(target_os = "windows"))]
fn launch_gui_exe_and_wait(_exe: &Path) -> Result<Option<i32>, String> {
    Err("仅 Windows 支持".into())
}

/// 启动官方有界面安装器并等待退出，再探测端点。
pub fn run_official_setup_installer() -> Result<VoiceEnvActionResult, String> {
    let zip = find_driver_zip().ok_or_else(|| "内嵌 VB-CABLE 驱动包不可用".to_string())?;
    let setup = stage_official_setup(&zip)?;
    log::info!(
        "XIAOMI VOICE ENV: launching official setup UI (ShellExecute) setup={}",
        setup.display()
    );

    let code = launch_gui_exe_and_wait(&setup)?;
    log::info!("XIAOMI VOICE ENV: official setup exited code={code:?}");

    invalidate_cable_probe_cache();
    for _ in 0..20 {
        let (i, o) = probe_cable_endpoints(true);
        if i && o {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    let (cable_input, cable_output) = probe_cable_endpoints(true);
    let ready = cable_endpoints_ready(cable_input, cable_output);

    let needs_reboot = !ready && (code == Some(3010) || code == Some(1641));

    let message = if ready {
        "官方 VB-CABLE 安装完成，CABLE Input 和 CABLE Output 均已检测到。请在外部输入法中选择 CABLE Output。".into()
    } else if needs_reboot {
        "官方安装程序已结束，但端点尚未出现。请按安装器提示重启 Windows，然后重新检测。".into()
    } else if code == Some(0) {
        "官方安装程序已退出，但仍未检测到 CABLE Input/Output。请检查设备管理器或使用官网安装包。"
            .into()
    } else {
        format!(
            "官方安装程序退出码 {code:?}。若已取消 UAC/安装，可再试；仍失败请用官网包手动安装。"
        )
    };

    Ok(VoiceEnvActionResult {
        ok: installer_observed_ready(code, cable_input, cable_output),
        ready,
        needs_choice: false,
        needs_reboot: needs_reboot && !ready,
        message,
        report_path: None,
    })
}

#[cfg(target_os = "windows")]
fn probe_via_cpal() -> (bool, bool) {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let mut cable_input = false;
    let mut cable_output = false;
    if let Ok(devices) = host.output_devices() {
        for d in devices {
            if let Ok(name) = d.name() {
                if name.to_ascii_lowercase().contains("cable input") {
                    cable_input = true;
                }
            }
        }
    }
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            if let Ok(name) = d.name() {
                if name.to_ascii_lowercase().contains("cable output") {
                    cable_output = true;
                }
            }
        }
    }
    (cable_input, cable_output)
}

#[cfg(target_os = "windows")]
fn mmdevices_endpoint_label(props: &winreg::RegKey) -> String {
    // 对齐 configure-xiaomi-audio.ps1 / Python native_audio
    const PKEY_DEVICE: &str = "{a45c254e-df1c-4efd-8020-67d146a850e0},2";
    const PKEY_ENDPOINT: &str = "{b3f8fa53-0004-438e-9003-51a46e139bfc},6";
    let mut parts: Vec<String> = Vec::new();
    for key in [PKEY_DEVICE, PKEY_ENDPOINT] {
        if let Ok(v) = props.get_value::<String, _>(key) {
            let t = v.trim();
            if !t.is_empty() && !parts.iter().any(|p| p.eq_ignore_ascii_case(t)) {
                parts.push(t.to_string());
            }
        }
    }
    parts.join(" ")
}

/// 对齐脚本：只认 DeviceState==1（Active）且名称匹配的端点。
#[cfg(target_os = "windows")]
fn registry_flow_has_cable(flow: &str, needle: &str) -> Result<bool, String> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    let path = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\{flow}");
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let root = hklm
        .open_subkey_with_flags(&path, KEY_READ)
        .map_err(|e| format!("open {path}: {e}"))?;

    let needle = needle.to_ascii_lowercase();
    for endpoint_id in root.enum_keys().filter_map(|k| k.ok()) {
        let Ok(endpoint) = root.open_subkey_with_flags(&endpoint_id, KEY_READ) else {
            continue;
        };
        let state: u32 = match endpoint.get_value("DeviceState") {
            Ok(v) => v,
            Err(_) => continue,
        };
        // DEVICE_STATE_ACTIVE = 1
        if state != 1 {
            continue;
        }
        let Ok(props) = endpoint.open_subkey_with_flags("Properties", KEY_READ) else {
            continue;
        };
        let label = mmdevices_endpoint_label(&props);
        if label.to_ascii_lowercase().contains(&needle) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(target_os = "windows")]
fn probe_via_registry() -> Result<(bool, bool), String> {
    let cable_input = registry_flow_has_cable("Render", "cable input")?;
    let cable_output = registry_flow_has_cable("Capture", "cable output")?;
    Ok((cable_input, cable_output))
}

#[cfg(target_os = "windows")]
fn probe_cable_endpoints_uncached() -> (bool, bool) {
    match probe_via_registry() {
        Ok(v) => v,
        Err(e) => {
            log::warn!("VB-CABLE registry probe failed ({e}); cpal fallback once");
            probe_via_cpal()
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn probe_cable_endpoints_uncached() -> (bool, bool) {
    (false, false)
}

/// 未就绪时的自动重试间隔。`REMOTE_BRIDGE_CABLE_PROBE_TTL_MS`：默认 60000；`0` = 不自动重试。
fn not_ready_retry_interval() -> Option<Duration> {
    match std::env::var("REMOTE_BRIDGE_CABLE_PROBE_TTL_MS") {
        Ok(s) => {
            let s = s.trim();
            if s.is_empty() {
                return Some(Duration::from_secs(60));
            }
            match s.parse::<u64>() {
                Ok(0) => None,
                Ok(ms) => Some(Duration::from_millis(ms)),
                Err(_) => Some(Duration::from_secs(60)),
            }
        }
        Err(_) => Some(Duration::from_secs(60)),
    }
}

struct CableProbeCache {
    at: Instant,
    cable_input: bool,
    cable_output: bool,
}

static CABLE_PROBE: Mutex<Option<CableProbeCache>> = Mutex::new(None);

fn probe_cable_endpoints(force: bool) -> (bool, bool) {
    if !force {
        let g = CABLE_PROBE.lock();
        if let Some(c) = g.as_ref() {
            let ready = cable_endpoints_ready(c.cable_input, c.cable_output);
            let cache_ttl = if ready {
                Duration::from_secs(2)
            } else {
                match not_ready_retry_interval() {
                    Some(interval) => interval,
                    None => return (c.cable_input, c.cable_output),
                }
            };
            if c.at.elapsed() < cache_ttl {
                return (c.cable_input, c.cable_output);
            }
        }
    }
    let (cable_input, cable_output) = probe_cable_endpoints_uncached();
    *CABLE_PROBE.lock() = Some(CableProbeCache {
        at: Instant::now(),
        cable_input,
        cable_output,
    });
    log::debug!("VB-CABLE probe input={cable_input} output={cable_output} force={force}");
    (cable_input, cable_output)
}

/// 安装/修复后立刻失效缓存，下次 status 会重探
pub fn invalidate_cable_probe_cache() {
    *CABLE_PROBE.lock() = None;
}

pub fn voice_env_status() -> VoiceEnvStatus {
    // Polling uses a short observation cache; explicit status/repair commands
    // use voice_env_status_fresh() and bypass it.
    voice_env_status_inner(false)
}

/// 用户主动「检测/修复」时强制重探
pub fn voice_env_status_fresh() -> VoiceEnvStatus {
    voice_env_status_inner(true)
}

fn voice_env_status_inner(force: bool) -> VoiceEnvStatus {
    let (cable_input, cable_output) = probe_cable_endpoints(force);
    let ready = cable_endpoints_ready(cable_input, cable_output);
    let zip = find_driver_zip();
    let embedded_available = zip.is_some();
    let message = if ready {
        "VB-CABLE 已就绪。请在外部输入法中自行选择 CABLE Output。".into()
    } else if embedded_available {
        "未检测到 VB-CABLE。可使用内嵌官方安装包，或打开官网下载最新版。".into()
    } else {
        "未检测到 VB-CABLE，且内嵌驱动包不可用。请从官网下载安装。".into()
    };
    VoiceEnvStatus {
        ready,
        cable_input,
        cable_output,
        embedded_available,
        embedded_zip_path: zip.map(|p| p.display().to_string()),
        download_page_url: DOWNLOAD_PAGE_URL.into(),
        download_zip_url: DOWNLOAD_ZIP_URL.into(),
        message,
    }
}

/// 检测当前端点；已就绪时不修改系统音频设置，未就绪时启动官方安装器。
pub fn check_or_prompt() -> VoiceEnvActionResult {
    let status = voice_env_status_fresh();
    if status.ready {
        return VoiceEnvActionResult {
            ok: true,
            ready: true,
            needs_choice: false,
            needs_reboot: false,
            message: "CABLE Input 和 CABLE Output 已就绪。请在外部输入法中选择 CABLE Output。"
                .into(),
            report_path: None,
        };
    }

    match run_official_setup_installer() {
        Ok(r) => r,
        Err(e) => VoiceEnvActionResult {
            ok: false,
            ready: false,
            needs_choice: true,
            needs_reboot: false,
            message: e,
            report_path: None,
        },
    }
}

pub fn install_embedded() -> Result<VoiceEnvActionResult, String> {
    run_official_setup_installer()
}

/// 与 install_embedded 相同（始终打开官方 Setup；保留 IPC 兼容）
pub fn install_embedded_force() -> Result<VoiceEnvActionResult, String> {
    run_official_setup_installer()
}

pub fn open_download_page() -> Result<VoiceEnvActionResult, String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", DOWNLOAD_PAGE_URL])
            .spawn()
            .map_err(|e| format!("打开下载页失败: {e}"))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        return Err("仅 Windows 支持".into());
    }
    Ok(VoiceEnvActionResult {
        ok: true,
        ready: false,
        needs_choice: false,
        needs_reboot: false,
        message: "已打开 VB-Audio 官网。安装完成后请再点「虚拟声卡修复」。".into(),
        report_path: None,
    })
}

pub fn open_download_zip() -> Result<VoiceEnvActionResult, String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", DOWNLOAD_ZIP_URL])
            .spawn()
            .map_err(|e| format!("打开下载链接失败: {e}"))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        return Err("仅 Windows 支持".into());
    }
    Ok(VoiceEnvActionResult {
        ok: true,
        ready: false,
        needs_choice: false,
        needs_reboot: false,
        message: "已开始下载官方驱动包。安装完成后请再点「虚拟声卡修复」。".into(),
        report_path: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{cable_endpoints_ready, installer_observed_ready};

    #[test]
    fn both_cable_endpoints_are_required() {
        assert!(!cable_endpoints_ready(false, false));
        assert!(!cable_endpoints_ready(true, false));
        assert!(!cable_endpoints_ready(false, true));
        assert!(cable_endpoints_ready(true, true));
    }

    #[test]
    fn installer_exit_zero_without_endpoints_is_not_success() {
        assert!(!installer_observed_ready(Some(0), false, false));
        assert!(installer_observed_ready(Some(1), true, true));
    }
}
