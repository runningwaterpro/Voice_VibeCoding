<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { storeToRefs } from "pinia";
import { useBridgeStore } from "../stores/bridge";
import { useAppUpdateStore } from "../stores/appUpdate";
import { useGlobalSettingsStore } from "../stores/globalSettings";
import SettingsSheet from "./SettingsSheet.vue";
import type { BridgeStatus } from "../types";

const route = useRoute();
const router = useRouter();
const bridge = useBridgeStore();
const appUpdate = useAppUpdateStore();
const globalSettings = useGlobalSettingsStore();
const { hideDevMenus } = storeToRefs(globalSettings);
const { updateInfo, shouldShowPassivePrompt } = storeToRefs(appUpdate);

const showQuitConfirm = ref(false);
const quitting = ref(false);
const showSettingsSheet = ref(false);

/** 顶栏会话摘要：来自主机状态（阶段 A） */
const session = ref({
  tone: "idle" as "idle" | "ok" | "warn" | "fail",
  title: "正在启动…",
  sub: "",
});
let hostTimer: ReturnType<typeof setInterval> | null = null;

function statusClass(status: BridgeStatus): string {
  if (status === "Connected") return "connected";
  if (status === "Connecting") return "connecting";
  if (status.startsWith("Error")) return "error";
  return "disconnected";
}

const allDeviceItems = [
  { path: "/xiaomi", label: "小米遥控器", type: "xiaomi" as const },
  { path: "/t1", label: "T1 [开发中]", type: "t1" as const, dev: true },
  { path: "/v60", label: "V60 [开发中]", type: "hanvon" as const, dev: true },
];

const deviceItems = computed(() =>
  hideDevMenus.value
    ? allDeviceItems.filter((item) => !item.dev)
    : allDeviceItems
);

const xiaomiStatusText = computed(() => bridge.statusLabel(bridge.devices.xiaomi.status));

const batteryRaw = computed(() => bridge.devices.xiaomi.battery_level);
const batteryText = computed(() =>
  batteryRaw.value == null ? "—" : `${Math.round(batteryRaw.value)}%`,
);
const batteryTone = computed(() => {
  const v = batteryRaw.value;
  if (v == null) return "unknown";
  if (v <= 15) return "crit";
  if (v <= 30) return "low";
  return "ok";
});

async function refreshSession() {
  try {
    const h = await invoke<{
      bridge_alive: boolean;
      atvv_ok: boolean;
      cable_ready: boolean;
      winuhid_ready: boolean;
      status_text: string;
      tone: string;
    }>("get_xiaomi_host_status");
    if (!h.bridge_alive) {
      session.value = { tone: "idle", title: "未连接遥控器", sub: "" };
      return;
    }
    const voiceOk = h.atvv_ok && h.cable_ready && h.winuhid_ready;
    if (voiceOk) {
      session.value = { tone: "ok", title: "语音可用", sub: "" };
    } else {
      session.value = {
        tone: h.tone === "error" ? "fail" : "warn",
        title: h.status_text || "语音未就绪",
        sub: "",
      };
    }
  } catch {
    session.value = { tone: "idle", title: xiaomiStatusText.value, sub: "" };
  }
}

const appVersion = ref("…");

onMounted(async () => {
  if (!globalSettings.loaded) {
    await globalSettings.load();
  }
  try {
    appVersion.value = `v${await getVersion()}`;
  } catch {
    appVersion.value = "v1.6.0";
  }
  await bridge.refreshStatus("xiaomi");
  await refreshSession();
  hostTimer = setInterval(() => {
    void refreshSession();
    void bridge.refreshStatus("xiaomi");
  }, 2000);
});

onUnmounted(() => {
  if (hostTimer) clearInterval(hostTimer);
});

function navigate(path: string) {
  router.push(path);
}

function isActive(path: string) {
  return route.path === path || route.path.startsWith(path + "/");
}

function openQuitConfirm() {
  showQuitConfirm.value = true;
}

function cancelQuit() {
  if (quitting.value) return;
  showQuitConfirm.value = false;
}

async function confirmQuit() {
  if (quitting.value) return;
  quitting.value = true;
  try {
    await invoke("quit_application");
  } catch (e) {
    console.error("quit_application failed:", e);
    quitting.value = false;
  }
}
</script>

<template>
  <header class="topnav">
    <div class="brand">
      <span class="brand-name">Voice VibeCoding</span>
      <span class="brand-ver">{{ appVersion }}</span>
      <template v-if="shouldShowPassivePrompt">
        <span class="brand-update-badge">新版本 V{{ updateInfo!.latestVersion }}</span>
        <button type="button" class="brand-update-btn" @click="appUpdate.openModal()">
          查看更新内容
        </button>
      </template>
    </div>

    <div class="session-chip" role="status" aria-live="polite">
      <span :class="['session-dot', `tone-${session.tone}`]" />
      <span class="session-title">{{ session.title }}</span>
      <span v-if="session.sub" class="session-sub">{{ session.sub }}</span>
    </div>

    <div
      class="battery-chip"
      :class="`is-${batteryTone}`"
      title="遥控器电池电量"
      aria-label="遥控器电池"
    >
      <svg class="battery-ico" viewBox="0 0 24 24" aria-hidden="true">
        <rect x="2" y="7" width="17" height="10" rx="2" fill="none" stroke="currentColor" stroke-width="1.5" />
        <path d="M21 10v4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        <rect class="battery-fill" x="4" y="9" width="10" height="6" rx="1" fill="currentColor" />
      </svg>
      <span>{{ batteryText }}</span>
    </div>

    <nav class="nav-row" v-if="deviceItems.length > 1">
      <button
        v-for="item in deviceItems"
        :key="item.path"
        type="button"
        :class="['nav-item', { active: isActive(item.path) }]"
        @click="navigate(item.path)"
      >
        <span
          :class="['dot', statusClass(bridge.devices[item.type].status)]"
          :title="bridge.statusLabel(bridge.devices[item.type].status)"
        />
        <span class="nav-label">{{ item.label }}</span>
      </button>
    </nav>

    <div class="nav-actions">
      <button
        type="button"
        class="nav-item"
        @click="showSettingsSheet = true"
      >
        <span class="nav-label">设置</span>
      </button>
      <button type="button" class="nav-item nav-exit" @click="openQuitConfirm">
        <span class="nav-label">退出</span>
      </button>
    </div>
  </header>

  <SettingsSheet v-if="showSettingsSheet" @close="showSettingsSheet = false" />

  <Teleport to="body">
    <div
      v-if="showQuitConfirm"
      class="quit-backdrop"
      role="presentation"
      @click.self="cancelQuit"
    >
      <div class="quit-dialog" role="dialog" aria-modal="true" aria-labelledby="quit-title">
        <h3 id="quit-title">退出应用？</h3>
        <p>将彻底关闭软件（不会最小化到托盘）。确定要退出吗？</p>
        <div class="quit-actions">
          <button type="button" class="quit-btn quit-btn-ghost" :disabled="quitting" @click="cancelQuit">
            取消
          </button>
          <button type="button" class="quit-btn quit-btn-danger" :disabled="quitting" @click="confirmQuit">
            {{ quitting ? "退出中..." : "退出" }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.topnav {
  display: flex;
  align-items: center;
  gap: 16px;
  height: 44px;
  padding: 0 16px;
  background: var(--sidebar-bg);
  color: var(--sidebar-text);
  user-select: none;
  flex-shrink: 0;
  min-width: 0;
  /* 与机身圆角对齐：父级 border-radius 不裁子元素 */
  border-radius: 11px 11px 0 0;
}

.session-chip {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex: 1;
  font-size: 12.5px;
}
.session-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  background: #64748b;
}
.session-dot.tone-ok {
  background: #34d399;
  box-shadow: 0 0 6px rgba(52, 211, 153, 0.7);
}
.session-dot.tone-warn {
  background: #fbbf24;
  box-shadow: 0 0 6px rgba(251, 191, 36, 0.6);
}
.session-dot.tone-fail {
  background: #f87171;
  box-shadow: 0 0 6px rgba(248, 113, 113, 0.6);
}
.session-title {
  font-weight: 600;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.session-sub {
  color: #94a3b8;
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.battery-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  flex-shrink: 0;
  padding: 3px 8px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: var(--panel);
  color: var(--text-secondary);
  font-size: 11.5px;
  font-family: "Cascadia Mono", ui-monospace, Consolas, monospace;
  transition: transform 160ms cubic-bezier(0.23, 1, 0.32, 1);
}
.battery-chip .battery-ico {
  width: 14px;
  height: 14px;
  color: var(--success);
}
.battery-chip.is-low .battery-ico {
  color: var(--warning);
}
.battery-chip.is-crit .battery-ico {
  color: var(--danger);
}
.battery-chip.is-unknown .battery-ico {
  color: var(--dim, #5c6673);
}
.single-device-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 12px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--sidebar-text);
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
}
.single-device-label:hover {
  background: #ffffff14;
  color: #fff;
}
.single-device-label.active {
  background: var(--sidebar-active);
  color: #fff;
}
.nav-connect.busy {
  opacity: 0.6;
  cursor: wait;
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.brand-name {
  font-size: 13px;
  font-weight: 600;
  color: #fff;
  letter-spacing: 0.01em;
  white-space: nowrap;
}

.brand-ver {
  font-size: 11px;
  color: #94a3b8;
}

.brand-update-badge {
  font-size: 11px;
  font-weight: 600;
  color: #93c5fd;
  white-space: nowrap;
}

.brand-update-btn {
  height: 24px;
  padding: 0 8px;
  border: 1px solid rgba(147, 197, 253, 0.45);
  border-radius: 4px;
  background: rgba(59, 130, 246, 0.15);
  color: #bfdbfe;
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.15s ease, color 0.15s ease;
}

.brand-update-btn:hover {
  background: rgba(59, 130, 246, 0.28);
  color: #fff;
}

.nav-row {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  overflow-x: auto;
  flex: 1;
}

.nav-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-left: auto;
  flex-shrink: 0;
}

.nav-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 12px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--sidebar-text);
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
  transition: background 0.15s cubic-bezier(0.23, 1, 0.32, 1),
    color 0.15s cubic-bezier(0.23, 1, 0.32, 1),
    transform 160ms cubic-bezier(0.23, 1, 0.32, 1);
}

.nav-item:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
}

.nav-item:active:not(:disabled) {
  transform: scale(0.97);
}

.nav-item.active {
  background: var(--sidebar-active);
  color: #fff;
}

.nav-exit:hover {
  background: rgba(239, 68, 68, 0.18);
  color: #fecaca;
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
  background: #64748b;
}

.dot.connected {
  background: var(--success);
}

.dot.connecting {
  background: var(--warning);
}

.dot.error {
  background: var(--danger);
}

.dot.disconnected {
  background: #64748b;
}

.quit-backdrop {
  position: fixed;
  inset: 0;
  z-index: 3000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: rgba(15, 23, 42, 0.45);
}

.quit-dialog {
  width: min(360px, 100%);
  padding: 18px 18px 14px;
  border-radius: 10px;
  background: var(--card-bg);
  box-shadow: 0 12px 40px rgba(15, 23, 42, 0.25);
  color: var(--text, #1e293b);
}

.quit-dialog h3 {
  margin: 0 0 8px;
  font-size: 16px;
  font-weight: 600;
}

.quit-dialog p {
  margin: 0 0 16px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-secondary, #64748b);
}

.quit-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.quit-btn {
  height: 32px;
  padding: 0 14px;
  border-radius: 6px;
  border: 1px solid transparent;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
}

.quit-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.quit-btn-ghost {
  background: var(--card-bg);
  border-color: var(--border, #e2e8f0);
  color: var(--text, #1e293b);
}

.quit-btn-ghost:hover:not(:disabled) {
  background: var(--panel-2);
}

.quit-btn-danger {
  background: var(--danger);
  color: #fff;
}

.quit-btn-danger:hover:not(:disabled) {
  filter: brightness(0.95);
}

.quit-btn:active:not(:disabled) {
  transform: scale(0.97);
}

@media (prefers-reduced-motion: reduce) {
  .nav-item,
  .nav-item:active,
  .nav-btn,
  .battery-chip,
  .quit-btn,
  .quit-btn:active {
    transition: none !important;
    transform: none !important;
  }
}
</style>
