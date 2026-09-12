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
        <button type="button" class="settings-close" aria-label="关闭设置" @click="emit('close')">
          ×
        </button>
      </header>

      <div class="settings-list">
        <label class="setting-row">
          <span class="setting-lab">
            <b>开机自启</b>
            <span>登录 Windows 后自动启动</span>
          </span>
          <input
            type="checkbox"
            class="settings-check"
            v-model="settings.autostart"
            @change="globalSettings.save()"
          />
        </label>
        <label class="setting-row">
          <span class="setting-lab">
            <b>启动后最小化到托盘</b>
            <span>不显示主窗口，托盘图标就绪</span>
          </span>
          <input
            type="checkbox"
            class="settings-check"
            v-model="settings.start_minimized_to_tray"
            @change="globalSettings.save()"
          />
        </label>
        <label class="setting-row">
          <span class="setting-lab">
            <b>最小化到托盘</b>
            <span>点关闭时进托盘，而不是退出</span>
          </span>
          <input
            type="checkbox"
            class="settings-check"
            v-model="settings.minimize_to_tray"
            @change="globalSettings.save()"
          />
        </label>
        <label class="setting-row">
          <span class="setting-lab">
            <b>隐藏开发中项目菜单</b>
            <span>仅显示小米遥控器</span>
          </span>
          <input
            type="checkbox"
            class="settings-check"
            v-model="settings.hide_dev_menus"
            @change="globalSettings.save()"
          />
        </label>
      </div>

      <footer class="settings-foot">
        <button type="button" class="settings-btn primary" @click="emit('close')">关闭</button>
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
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px 18px 14px;
  color: var(--text);
}
.settings-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}
.settings-head h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}
.settings-close {
  width: 28px;
  height: 28px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel-2);
  color: var(--text-secondary);
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
}
.settings-close:hover {
  color: var(--text);
  background: var(--surface-hover);
}
.settings-list {
  display: flex;
  flex-direction: column;
}
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 0;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
}
.setting-row:last-child {
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
.settings-check {
  width: 40px;
  height: 22px;
  accent-color: var(--primary);
  cursor: pointer;
}
.settings-foot {
  display: flex;
  justify-content: flex-end;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--border);
}
.settings-btn {
  min-height: 30px;
  padding: 0 14px;
  border-radius: 7px;
  border: 1px solid var(--border);
  background: var(--panel-2);
  color: var(--text);
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
}
.settings-btn.primary {
  border-color: var(--primary);
  color: #0f172a;
  background: var(--primary);
}
</style>
