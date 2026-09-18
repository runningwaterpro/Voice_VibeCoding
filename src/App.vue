<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { RouterView, useRouter } from "vue-router";
import { listen, emit, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import SideNav from "./components/SideNav.vue";
import WindowTitlebar from "./components/WindowTitlebar.vue";
import AppUpdateModal from "./components/AppUpdateModal.vue";
import { useAppUpdateStore } from "./stores/appUpdate";
import { useGlobalSettingsStore } from "./stores/globalSettings";

const appUpdate = useAppUpdateStore();
const globalSettings = useGlobalSettingsStore();

interface ConflictProcess {
  pid: number;
  name: string;
  reasons: string[];
}

interface ConflictSnapshot {
  trigger: string;
  detail: string;
  processes: ConflictProcess[];
  pcmPort: number;
  hidTapPort: number;
}

const router = useRouter();
let unlistenNav: UnlistenFn | null = null;
let unlistenConflict: UnlistenFn | null = null;

// WebView2 健康心跳：页面 JS 存活时每 5s 报告一次，供后端判定渲染进程是否死亡（自动 reload）
let heartbeatTimer: ReturnType<typeof setInterval> | null = null;

const showConflict = ref(false);
const conflict = ref<ConflictSnapshot | null>(null);
const busy = ref(false);
const actionMsg = ref("");

function triggerLabel(t: string): string {
  switch (t) {
    case "pcm_port":
      return "语音端口冲突";
    case "hid_tap_port":
      return "HID Tap 端口冲突";
    case "atvv":
      return "ATVV 语音通道失败";
    case "atvv_repair":
      return "修复 ATVV：请先结束占用";
    default:
      return "桥接进程冲突";
  }
}

function openConflict(snap: ConflictSnapshot) {
  if (!snap.processes?.length) return;
  conflict.value = snap;
  actionMsg.value = "";
  showConflict.value = true;
}

async function killOne(pid: number) {
  if (busy.value || !conflict.value) return;
  busy.value = true;
  actionMsg.value = "";
  try {
    await invoke<number[]>("kill_xiaomi_conflicts", { pids: [pid] });
    conflict.value.processes = conflict.value.processes.filter((p) => p.pid !== pid);
    if (conflict.value.processes.length === 0) {
      await autoRetry();
    }
  } catch (e) {
    actionMsg.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function killAll() {
  if (busy.value || !conflict.value) return;
  const pids = conflict.value.processes.map((p) => p.pid);
  if (!pids.length) return;
  busy.value = true;
  actionMsg.value = "";
  try {
    await invoke<number[]>("kill_xiaomi_conflicts", { pids });
    conflict.value.processes = [];
    await autoRetry();
  } catch (e) {
    actionMsg.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function autoRetry() {
  const trigger = conflict.value?.trigger ?? "";
  try {
    const msg = await invoke<string>("retry_xiaomi_after_conflict_clear");
    actionMsg.value = msg;
    showConflict.value = false;
    if (trigger === "atvv_repair") {
      actionMsg.value = "占用已清理，正在继续修复 ATVV…";
      try {
        const result = await invoke<{
          phase: string;
          message: string;
          atvvOk: boolean;
        }>("repair_xiaomi_atvv", { force: true });
        actionMsg.value = result.message;
      } catch (e) {
        actionMsg.value = String(e);
      }
    }
  } catch (e) {
    actionMsg.value = String(e);
  }
}

async function dismissConflict() {
  const trigger = conflict.value?.trigger;
  showConflict.value = false;
  if (trigger === "atvv_repair") {
    await emit("xiaomi-atvv-repair-cancelled", {
      message: "已取消：未结束占用进程，ATVV 修复中止",
    });
  }
}

/** 自动贴合窗口高度：量「自然内容高」（不受 flex 撑满影响），可增可减 */
function measureNaturalContentHeight(): number {
  const titlebar = document.querySelector<HTMLElement>(".titlebar");
  const topnav = document.querySelector<HTMLElement>(".topnav");
  const main = document.querySelector<HTMLElement>(".main-content");
  if (!main) return 0;

  const titlebarH = Math.ceil(titlebar?.getBoundingClientRect().height ?? 36);
  const topnavH = Math.ceil(topnav?.getBoundingClientRect().height ?? 40);

  // 临时取消 flex 撑满，量主内容自然高（含 padding）
  const prevFlex = main.style.flex;
  const prevHeight = main.style.height;
  const prevOverflow = main.style.overflowY;
  main.style.flex = "0 0 auto";
  main.style.height = "auto";
  main.style.overflowY = "visible";
  const mainH = Math.ceil(main.getBoundingClientRect().height);
  main.style.flex = prevFlex;
  main.style.height = prevHeight;
  main.style.overflowY = prevOverflow;

  return titlebarH + topnavH + mainH;
}

async function fitWindowHeightToContent() {
  try {
    const win = getCurrentWindow();
    if (await win.isMaximized()) return;

    await new Promise((r) =>
      requestAnimationFrame(() => requestAnimationFrame(r)),
    );

    const contentH = measureNaturalContentHeight();
    if (contentH < 200) return;

    const scale = (await win.scaleFactor()) || 1;
    const inner = await win.innerSize();
    const curW = Math.round(inner.width / scale);
    const curInnerH = Math.round(inner.height / scale);

    // setSize = 内容区（inner）高度；与 tauri.conf.json max/minHeight 一致
    const maxH = 900;
    const height = Math.min(maxH, Math.max(contentH + 4, 480));

    // 与目标差 ≤4px 不动，避免抖动
    if (Math.abs(curInnerH - height) <= 4) return;
    await win.setSize(new LogicalSize(curW, height));
  } catch (e) {
    console.warn("fit window height failed:", e);
  }
}

function scheduleFitWindowHeight() {
  void fitWindowHeightToContent();
  for (const d of [500, 1500, 3000, 5000]) {
    setTimeout(() => {
      void fitWindowHeightToContent();
    }, d);
  }
}

onMounted(async () => {
  // 页面就绪后再显示；启动策略为托盘时由后端 minimize（禁止 hide，防 WebView2 白屏）
  try {
    await invoke("reveal_main_on_frontend_ready");
  } catch (e) {
    // 不盲目 show()：开启「启动后最小化到托盘」时 fallback show 会误弹窗；用户可点托盘打开
    console.warn("reveal main window failed:", e);
  }
  scheduleFitWindowHeight();
  await appUpdate.init();
  if (!globalSettings.loaded) {
    await globalSettings.load();
  }
  unlistenNav = await listen<string>("navigate", (ev) => {
    if (ev.payload) router.push(ev.payload);
  });
  try {
    unlistenConflict = await listen<ConflictSnapshot>("xiaomi-conflict", (ev) => {
      if (ev.payload) openConflict(ev.payload);
    });
  } catch (e) {
    console.warn("listen xiaomi-conflict failed:", e);
  }
  // 健康心跳（与后端 webview-guard 守卫线程配合）
  heartbeatTimer = setInterval(() => {
    invoke("webview_ping").catch(() => {
      /* 渲染进程已死时 invoke 必然失败，交给后端守卫处理 */
    });
  }, 5000);
});

onUnmounted(() => {
  unlistenNav?.();
  unlistenConflict?.();
  appUpdate.dispose();
  if (heartbeatTimer) clearInterval(heartbeatTimer);
});
</script>

<template>
  <div class="app-container">
    <WindowTitlebar />
    <SideNav />
    <main class="main-content">
      <RouterView />
    </main>
  </div>

  <Teleport to="body">
    <div
      v-if="showConflict && conflict"
      class="conflict-backdrop"
      role="presentation"
      @click.self="dismissConflict"
    >
      <div
        class="conflict-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="conflict-title"
      >
        <h3 id="conflict-title">{{ triggerLabel(conflict.trigger) }}</h3>
        <p class="conflict-detail">
          {{ conflict.detail || "检测到其它遥控桥接进程，可能占用端口或 BLE。" }}
        </p>
        <p class="conflict-ports">
          关注端口：PCM UDP {{ conflict.pcmPort }}、HID Tap TCP {{ conflict.hidTapPort }}
        </p>

        <ul class="conflict-list">
          <li v-for="p in conflict.processes" :key="p.pid" class="conflict-item">
            <div class="conflict-item-main">
              <span class="conflict-name">{{ p.name }}</span>
              <span class="conflict-pid">PID {{ p.pid }}</span>
              <span class="conflict-reasons">{{ p.reasons.join(" · ") }}</span>
            </div>
            <button
              type="button"
              class="conflict-btn conflict-btn-row"
              :disabled="busy"
              @click="killOne(p.pid)"
            >
              结束此进程
            </button>
          </li>
        </ul>

        <p class="conflict-hint">
          也可手动打开任务管理器（Ctrl+Shift+Esc）结束上列进程。仅允许结束已知桥接程序。
        </p>

        <p v-if="actionMsg" class="conflict-msg">{{ actionMsg }}</p>

        <div class="conflict-actions">
          <button
            type="button"
            class="conflict-btn conflict-btn-ghost"
            :disabled="busy"
            @click="dismissConflict"
          >
            取消
          </button>
          <button
            type="button"
            class="conflict-btn conflict-btn-danger"
            :disabled="busy || !conflict.processes.length"
            @click="killAll"
          >
            {{ busy ? "处理中…" : "关掉上列全部" }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>

  <AppUpdateModal />
</template>

<style>
:root {
  --primary: #4db6a4;
  --primary-dark: #3a9a8a;
  --chassis: #171a1f;
  --panel: #1f242b;
  --panel-2: #262c35;
  --edge: #343b46;
  --bg: #171a1f;
  --sidebar-bg: #14171c;
  --sidebar-text: #e8ecf1;
  --sidebar-active: #4db6a4;
  --card-bg: #1f242b;
  --border: #343b46;
  --text: #e8ecf1;
  --text-secondary: #8b95a3;
  --success: #4db6a4;
  --warning: #d4a84b;
  --danger: #e06b6b;
  --radius: 10px;
  --surface-hover: #2a313b;
  --surface-muted: #262c35;
  --surface-selected: #243044;
  --focus-ring: rgba(59, 130, 246, 0.25);
  color-scheme: dark;
}

/* Dark form controls — overrides light component defaults */
input,
select,
textarea {
  background: var(--panel-2);
  border: 1px solid var(--edge);
  color: var(--text);
  border-radius: 6px;
}

input:focus,
select:focus,
textarea:focus {
  border-color: var(--primary);
  outline: none;
  box-shadow: 0 0 0 3px var(--focus-ring);
}

input::placeholder {
  color: var(--dim, #5c6673);
}

input[type="number"] {
  color: var(--text);
}

input[type="number"]::-webkit-outer-spin-button,
input[type="number"]::-webkit-inner-spin-button {
  opacity: 0.4;
}

/* Scrollbars */
::-webkit-scrollbar {
  width: 10px;
  height: 10px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: #3a4250;
  border-radius: 6px;
  border: 2px solid var(--chassis);
}
::-webkit-scrollbar-thumb:hover {
  background: #4a5565;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  /* 与机身同色：无边框窗边缘不留更深的「外框」 */
  background: var(--chassis);
  color: var(--text);
  overflow: hidden;
  min-height: 0;
  height: 100%;
}

#app {
  min-height: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
}

/* 无边框窗：机身铺满视口，不留 max-width/margin 悬浮卡片外框 */
.app-container {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: none;
  margin: 0;
  background: var(--chassis);
  border: none;
  border-radius: 0;
  box-shadow: none;
  flex: 1 1 auto;
  min-height: 0;
  overflow: hidden;
}

.main-content {
  flex: 1 1 auto;
  overflow-x: clip;
  overflow-y: auto;
  padding: 10px 12px 12px;
  background: var(--chassis);
  border-radius: 0;
  /* 内容不足时不靠底部留白撑开：对齐顶部，空白只来自窗口过高 */
  display: flex;
  flex-direction: column;
  align-items: stretch;
  justify-content: flex-start;
}

.conflict-backdrop {
  position: fixed;
  inset: 0;
  z-index: 4000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: rgba(0, 0, 0, 0.55);
}

.conflict-dialog {
  width: min(480px, 100%);
  padding: 18px 18px 14px;
  border-radius: 10px;
  background: var(--panel);
  border: 1px solid var(--edge);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
  color: var(--text);
}

.conflict-dialog h3 {
  margin: 0 0 8px;
  font-size: 16px;
  font-weight: 600;
}

.conflict-detail,
.conflict-ports,
.conflict-hint {
  margin: 0 0 10px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-secondary);
}

.conflict-list {
  list-style: none;
  margin: 0 0 12px;
  padding: 0;
  max-height: 220px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
}

.conflict-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
}

.conflict-item:last-child {
  border-bottom: none;
}

.conflict-item-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.conflict-name {
  font-size: 13px;
  font-weight: 600;
  word-break: break-all;
}

.conflict-pid,
.conflict-reasons {
  font-size: 12px;
  color: var(--text-secondary);
}

.conflict-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

.conflict-btn {
  height: 32px;
  padding: 0 14px;
  border-radius: 6px;
  border: 1px solid transparent;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
}

.conflict-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.conflict-btn-ghost {
  background: var(--panel-2);
  border-color: var(--border);
  color: var(--text);
}

.conflict-btn-ghost:hover:not(:disabled) {
  background: var(--surface-hover);
}

.conflict-btn-danger {
  background: var(--danger);
  color: #fff;
}

.conflict-btn-danger:hover:not(:disabled) {
  filter: brightness(0.95);
}

.conflict-btn-row {
  flex-shrink: 0;
  height: 28px;
  padding: 0 10px;
  background: var(--panel-2);
  border-color: var(--border);
  color: var(--text);
}

.conflict-btn-row:hover:not(:disabled) {
  border-color: var(--danger);
  color: var(--danger);
}

.conflict-msg {
  margin: 0 0 10px;
  font-size: 12px;
  color: var(--warning);
}
</style>
