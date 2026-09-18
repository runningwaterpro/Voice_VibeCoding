<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appVersion = ref("…");
const active = ref(true);

async function refreshActive() {
  try {
    active.value = await getCurrentWindow().isFocused();
  } catch {
    /* ignore */
  }
}

async function minimize() {
  try {
    await getCurrentWindow().minimize();
  } catch (e) {
    console.warn("minimize failed:", e);
  }
}

async function close() {
  try {
    await getCurrentWindow().close();
  } catch (e) {
    console.warn("close failed:", e);
  }
}

let unsub: Array<() => void> = [];

onMounted(async () => {
  try {
    appVersion.value = `v${await getVersion()}`;
  } catch {
    appVersion.value = "v1.1.1";
  }
  await refreshActive();
  try {
    const win = getCurrentWindow();
    const un = await win.onFocusChanged((e) => {
      active.value = !!(e as unknown as { focused?: boolean }).focused;
    });
    unsub = [un];
  } catch {
    /* optional events */
  }
});

onUnmounted(() => {
  unsub.forEach((u) => {
    try {
      u();
    } catch {
      /* ignore */
    }
  });
});
</script>

<template>
  <header
    class="titlebar"
    data-tauri-drag-region
    :data-active="active ? 'true' : 'false'"
  >
    <div class="brand" data-tauri-drag-region>
      <b>Voice VibeCoding</b>
      <span>{{ appVersion }}</span>
    </div>
    <div class="win-controls" role="group" aria-label="窗口控制">
      <button type="button" class="win-ctl" aria-label="最小化" title="最小化" @click="minimize">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8h10" /></svg>
      </button>
      <button type="button" class="win-ctl close" aria-label="关闭" title="关闭" @click="close">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" /></svg>
      </button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  display: flex;
  align-items: center;
  height: 36px;
  /* 比机身更深一档，与状态栏/内容分层可辨 */
  background: #0b0d11;
  flex-shrink: 0;
  position: sticky;
  top: 0;
  z-index: 30;
  border-radius: 0;
  border-bottom: 1px solid var(--edge, #343b46);
  box-shadow: inset 0 -1px 0 rgba(0, 0, 0, 0.45);
  padding-left: 14px;
  -webkit-app-region: drag;
  app-region: drag;
  user-select: none;
}

.titlebar[data-active="false"] {
  opacity: 0.88;
}

.brand {
  display: flex;
  align-items: baseline;
  gap: 8px;
  flex-shrink: 0;
  min-width: 0;
}

.brand b {
  font-size: 12.5px;
  font-weight: 600;
  color: #c5cdd8;
}

.brand span {
  font-size: 10.5px;
  color: #6b7685;
  font-family: "Cascadia Mono", ui-monospace, Consolas, monospace;
}

.win-controls {
  display: flex;
  align-items: center;
  gap: 0;
  margin-left: auto;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.win-ctl {
  width: 40px;
  height: 36px;
  border: none;
  background: transparent;
  color: #8b95a3;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  cursor: pointer;
  transition: background-color 120ms cubic-bezier(0.23, 1, 0.32, 1), color 120ms cubic-bezier(0.23, 1, 0.32, 1);
}

.win-ctl:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #e8ecf1;
}

.win-ctl:active {
  background: rgba(255, 255, 255, 0.12);
}

.win-ctl.close:hover {
  background: #e81123;
  color: #fff;
}

.win-ctl.close:active {
  background: #c50f1f;
}

.win-ctl svg {
  width: 16px;
  height: 16px;
  stroke: currentColor;
  fill: none;
  stroke-width: 1.5;
  stroke-linecap: round;
  stroke-linejoin: round;
}
</style>
