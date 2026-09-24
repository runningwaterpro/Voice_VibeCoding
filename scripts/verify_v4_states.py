"""Screenshot app rail states vs V4 demo (Tauri mocked)."""
from __future__ import annotations

import http.server
import json
import socketserver
import threading
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
DIST = ROOT / "dist"
DEMO = ROOT / "docs" / "design" / "ui-redesign-demo-v4.html"
OUT = ROOT / "scripts" / "v4-shots"
OUT.mkdir(exist_ok=True)

PORT = 8765

HOST_ITEMS = [
    {"id": "bridge", "label": "桥接进程", "state_label": "", "tone": "ok"},
    {"id": "ble", "label": "蓝牙", "state_label": "", "tone": "ok"},
    {"id": "atvv", "label": "语音通道", "state_label": "", "tone": "ok"},
    {"id": "cable", "label": "虚拟声卡", "state_label": "", "tone": "ok"},
    {"id": "hid", "label": "虚拟键盘", "state_label": "", "tone": "ok"},
]


def make_host(**kw):
    base = {
        "bridge_alive": True,
        "audio_alive": True,
        "cable_ready": True,
        "winuhid_ready": True,
        "atvv_ok": True,
        "status_text": "运行正常",
        "detail": "",
        "tone": "ok",
        "items": json.loads(json.dumps(HOST_ITEMS)),
    }
    base.update(kw)
    return base


def make_meter(**kw):
    base = {
        "ble_state": "session",
        "ble_level": 0.35,
        "waveform": [0.2] * 28,
        "cable_active": False,
        "cable_level": 0.2,
        "atvv_ok": True,
    }
    base.update(kw)
    return base


STATES = {
    "idle": (
        make_host(
            bridge_alive=False,
            audio_alive=False,
            cable_ready=False,
            winuhid_ready=False,
            atvv_ok=False,
            status_text="未连接遥控器",
            detail="桥接未运行。",
            tone="idle",
        ),
        make_meter(ble_state="idle", ble_level=0, cable_level=0, atvv_ok=False),
    ),
    "ready": (make_host(), make_meter()),
    "voice": (
        make_host(status_text="采音中", detail="正在采音送声。"),
        make_meter(ble_state="receiving", ble_level=0.55, cable_active=True, cable_level=0.8),
    ),
    "error_atvv": (
        make_host(
            atvv_ok=False,
            status_text="ATVV 未连接",
            detail="点「修复 ATVV 连接」。",
            tone="warn",
        ),
        make_meter(atvv_ok=False, cable_active=False, cable_level=0),
    ),
    "err_hid": (
        make_host(
            winuhid_ready=False,
            status_text="虚拟键盘未就绪",
            detail="点「修复虚拟键盘」。",
            tone="warn",
        ),
        make_meter(cable_active=False, cable_level=0),
    ),
    "err_cable": (
        make_host(
            cable_ready=False,
            status_text="语音环境未就绪",
            detail="点「虚拟声卡修复」。",
            tone="warn",
        ),
        make_meter(cable_active=False, cable_level=0),
    ),
    "err_route": (
        make_host(
            audio_alive=False,
            status_text="语音路由未就绪",
            detail="点「重启桥接」。",
            tone="warn",
        ),
        make_meter(cable_active=False, cable_level=0),
    ),
    "err_bridge": (
        make_host(
            bridge_alive=False,
            audio_alive=False,
            cable_ready=False,
            winuhid_ready=False,
            atvv_ok=False,
            status_text="桥接未运行",
            detail="点「重新连接」或「重启桥接」。",
            tone="error",
        ),
        make_meter(ble_state="idle", ble_level=0, cable_level=0, atvv_ok=False),
    ),
}

DEMO_MAP = {
    "idle": "idle",
    "ready": "ready",
    "voice": "voice",
    "error_atvv": "error",
    "err_hid": "err_hid",
    "err_cable": "err_cable",
    "err_route": "err_route",
    "err_bridge": "err_bridge",
}

INIT_TEMPLATE = """
window.__MOCK_HOST__ = %(host)s;
window.__MOCK_METER__ = %(meter)s;
window.__TAURI_INTERNALS__ = {
  invoke: async (cmd) => {
    if (cmd === 'get_xiaomi_host_status') return window.__MOCK_HOST__;
    if (cmd === 'get_device_status') return {
      type: 'xiaomi', name: 'Xiaomi Remote 2 Pro',
      bridge_type: 'xiaomi',
      status: window.__MOCK_HOST__.bridge_alive ? 'Connected' : 'Disconnected',
      device_name: 'Xiaomi Remote 2 Pro',
      device_address: '00:11:22:33:44:55',
      battery_level: 83,
      battery: null,
    };
    if (cmd === 'load_config') return {
      keys: {}, gain_db: 10, voice_shortcut: null,
      voice_shortcut_enabled: true, voice_trigger_mode: 'hold',
    };
    if (cmd === 'get_xiaomi_voice_meter') return window.__MOCK_METER__;
    if (cmd === 'get_app_update_status') return { available: false };
    if (cmd === 'get_global_settings') return {
      autostart: false, start_minimized: false, minimize_to_tray: true,
    };
    return null;
  },
  metadata: { currentWindow: { label: 'main' } },
};
"""


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(DIST), **kwargs)

    def log_message(self, format, *args):  # noqa: A003
        pass


def start_server():
    socketserver.TCPServer.allow_reuse_address = True
    httpd = socketserver.TCPServer(("127.0.0.1", PORT), Handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


EXTRACT = """
() => {
  const q = (s) => document.querySelector(s);
  const qa = (s) => [...document.querySelectorAll(s)];
  const text = (el) => (el ? el.textContent.trim().replace(/\\s+/g, ' ') : null);
  const visible = (el) => !!el && el.offsetParent !== null && getComputedStyle(el).display !== 'none';
  const chips = qa('.chip').filter(visible).map((c) => ({ cls: c.className, text: text(c) }));
  const attention = qa('.btn-attention').filter(visible).map(text);
  const attentionSec = qa('.btn-attention-sec').filter(visible).map(text);
  const healthy = q('.healthy-note');
  const statusLine = text(q('.status-line'));
  const conn = text(q('.rail-conn'));
  const moreOps = visible(q('.more-ops'));
  const topnavHasConnect = !!q('.nav-connect');
  const railBtns = qa('.status-col .btn, .host-card .btn').filter(visible).map((b) => ({
    text: text(b), cls: b.className,
  }));
  return {
    chips, attention, attentionSec,
    healthy: visible(healthy) ? text(healthy) : null,
    statusLine, conn, moreOps, topnavHasConnect, railBtns,
    battery: text(q('.battery-chip')),
    hasSettingsCloseX: !!q('.settings-close'),
    hasSwitch: !!q('.switch'),
  };
}
"""


def main():
    httpd = start_server()
    results = {}
    with sync_playwright() as p:
        browser = p.chromium.launch()
        ctx = browser.new_context(viewport={"width": 1200, "height": 900})

        demo_page = ctx.new_page()
        demo_page.goto(DEMO.as_uri())
        demo_page.wait_for_timeout(200)
        for name, demo_key in DEMO_MAP.items():
            demo_page.click(f'.demo-bar [data-state="{demo_key}"]')
            demo_page.wait_for_timeout(150)
            demo_page.screenshot(path=str(OUT / f"demo-{name}.png"), full_page=True)
        demo_page.close()

        for name, (host, meter) in STATES.items():
            init = INIT_TEMPLATE % {
                "host": json.dumps(host, ensure_ascii=False),
                "meter": json.dumps(meter, ensure_ascii=False),
            }
            page = ctx.new_page()
            page.add_init_script(init)
            page.goto(f"http://127.0.0.1:{PORT}/")
            page.wait_for_timeout(1400)
            results[name] = page.evaluate(EXTRACT)
            page.screenshot(path=str(OUT / f"app-{name}.png"), full_page=True)
            page.close()

        browser.close()
    httpd.shutdown()
    (OUT / "results.json").write_text(
        json.dumps(results, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print(json.dumps(results, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
