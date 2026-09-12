<script setup lang="ts">
import { onMounted } from "vue";
import { storeToRefs } from "pinia";
import { useGlobalSettingsStore } from "../stores/globalSettings";

const emit = defineEmits<{
  close: [];
}>();
const globalSettings = useGlobalSettingsStore();
const { settings } = storeToRefs(globalSettings);

onMounted(async () => {
  if (!globalSettings.loaded) {
    await globalSettings.load();
  }
});
</script>

<template>
  <div class="settings-backdrop" role="presentation" @click.self="emit('close')">
    <div class="settings-sheet" role="dialog" aria-modal="true" aria-labelledby="setTitle">
      <header class="settings-head">
        <h2 id="setTitle">设置</h2>
      </header>

      <div class="settings-list">
        <label class="setting-row">
          <span class="setting-lab">
            <b>开机自启</b>
            <span>登录 Windows 后自动启动</span>
          </span>
          <button
            type="button"
            class="switch"
            :class="{ on: settings.autostart }"
            :aria-pressed="settings.autostart"
            aria-label="开机自启"
            @click.prevent="settings.autostart = !settings.autostart; globalSettings.save()"
          >
            <span class="switch-knob" />
          </button>
        </label>
        <label class="setting-row">
          <span class="setting-lab">
            <b>启动后最小化到托盘</b>
            <span>不显示主窗口，托盘图标就绪</span>
          </span>
          <button
            type="button"
            class="switch"
            :class="{ on: settings.start_minimized_to_tray }"
            :aria-pressed="settings.start_minimized_to_tray"
            aria-label="启动后最小化到托盘"
            @click.prevent="settings.start_minimized_to_tray = !settings.start_minimized_to_tray; globalSettings.save()"
          >
            <span class="switch-knob" />
          </button>
        </label>
        <label class="setting-row">
          <span class="setting-lab">
            <b>最小化到托盘</b>
            <span>点关闭时进托盘，而不是退出</span>
          </span>
          <button
            type="button"
            class="switch"
            :class="{ on: settings.minimize_to_tray }"
            :aria-pressed="settings.minimize_to_tray"
            aria-label="最小化到托盘"
            @click.prevent="settings.minimize_to_tray = !settings.minimize_to_tray; globalSettings.save()"
          >
            <span class="switch-knob" />
          </button>
        </label>
        <label class="setting-row">
          <span class="setting-lab">
            <b>隐藏开发中项目菜单</b>
            <span>仅显示小米遥控器</span>
          </span>
          <button
            type="button"
            class="switch"
            :class="{ on: settings.hide_dev_menus }"
            :aria-pressed="settings.hide_dev_menus"
            aria-label="隐藏开发中项目菜单"
            @click.prevent="settings.hide_dev_menus = !settings.hide_dev_menus; globalSettings.save()"
          >
            <span class="switch-knob" />
          </button>
        </label>
      </div>

      <footer class="settings-foot">
        <button type="button" class="settings-btn close" @click="emit('close')">关闭</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.settings-backdrop {
  position: fixed;
  inset: 0;
  z-index: 2000;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding: 48px 16px;
  background: rgba(0, 0, 0, 0.55);
  overflow: auto;
}
.settings-sheet {
  width: min(520px, 100%);
  background: var(--panel);
  border: 1px solid var(--edge, #343b46);
  border-radius: 12px;
  padding: 16px 18px 14px;
  color: var(--text);
}
.settings-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.settings-head h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 9px 0;
  border-bottom: 1px solid #2a3038;
  cursor: default;
}
.setting-row:last-of-type {
  border-bottom: none;
}
.setting-lab {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.setting-lab b {
  font-weight: 500;
  font-size: 13.5px;
}
.setting-lab span {
  font-size: 12px;
  color: var(--text-secondary);
}
.switch {
  position: relative;
  width: 40px;
  height: 22px;
  border-radius: 99px;
  background: #3a424e;
  border: none;
  flex-shrink: 0;
  cursor: pointer;
  padding: 0;
  transition: background-color 150ms cubic-bezier(0.23, 1, 0.32, 1),
    transform 160ms cubic-bezier(0.23, 1, 0.32, 1);
}
.switch:active {
  transform: scale(0.97);
}
.switch-knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #c5cad1;
  transition: transform 150ms cubic-bezier(0.23, 1, 0.32, 1),
    background-color 150ms cubic-bezier(0.23, 1, 0.32, 1);
  pointer-events: none;
}
.switch.on {
  background: #2f6a5f;
}
.switch.on .switch-knob {
  transform: translateX(18px);
  background: var(--success, #4db6a4);
}
.settings-foot {
  display: flex;
  justify-content: flex-end;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid #2a3038;
}
.settings-btn {
  min-height: 28px;
  padding: 0 14px;
  border-radius: 6px;
  border: 1px solid var(--edge, #343b46);
  background: var(--panel-2, #262c35);
  color: var(--text-secondary);
  font-size: 12.5px;
  font-weight: 500;
  cursor: pointer;
  transition: transform 160ms cubic-bezier(0.23, 1, 0.32, 1),
    background-color 150ms cubic-bezier(0.23, 1, 0.32, 1),
    border-color 150ms cubic-bezier(0.23, 1, 0.32, 1),
    color 150ms cubic-bezier(0.23, 1, 0.32, 1);
}
.settings-btn:hover {
  color: var(--text);
  border-color: var(--text-secondary);
}
.settings-btn:active {
  transform: scale(0.97);
}
</style>
