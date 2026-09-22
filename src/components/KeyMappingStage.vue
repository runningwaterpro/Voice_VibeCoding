<script setup lang="ts">
import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  watch,
} from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DeviceConfig, KeyAction } from "../types";
import RemoteHotspot from "./RemoteHotspot.vue";
import RemoteKeyIcon from "./RemoteKeyIcon.vue";
import VoiceShortcutComposer from "./VoiceShortcutComposer.vue";
import { MEDIA_PICK_KEYS, vkDisplayName } from "../utils/vkDisplay";

const props = defineProps<{
  config: DeviceConfig;
  /** 实体键：与遥控同步——按下亮、按住保持、抬起灭 */
  pressPulse?: { id: string; seq: number; phase: "down" | "up" } | null;
}>();

const emit = defineEmits<{
  save: [config: DeviceConfig];
}>();

const LEFT_IDS = [
  "power",
  "up",
  "left",
  "ok",
  "back",
  "home",
  "menu",
] as const;
const RIGHT_IDS = [
  "mic",
  "right",
  "down",
  "volume_up",
  "volume_down",
  "tv",
] as const;

const DEFAULT_LABELS: Record<string, string> = {
  power: "电源",
  mic: "语音",
  up: "上",
  left: "左",
  ok: "确定",
  right: "右",
  down: "下",
  back: "返回",
  volume_up: "音量+",
  home: "主页",
  volume_down: "音量-",
  menu: "菜单",
  tv: "TV",
};

const selectedId = ref<string | null>(null);
const hoverId = ref<string | null>(null);
const pressedId = ref<string | null>(null);
let pressClearTimer: ReturnType<typeof setTimeout> | null = null;
const capturing = ref(false);
const captureError = ref<string | null>(null);
const liveLabels = ref<string[]>([]);
const manualEditor = ref<{ buttonId: string; initialKeys: number[] } | null>(null);

function isLeftButton(id: string): boolean {
  return (LEFT_IDS as readonly string[]).includes(id);
}

function openManualShortcutEditor() {
  const id = selectedId.value;
  if (!id) return;
  const action = props.config.button_bindings?.[id] || { type: "None", value: null };
  const keys = actionToVks(action) || [];
  manualEditor.value = { buttonId: id, initialKeys: keys };
  capturing.value = false;
  void cancelCapture();
  lineOpacity.value = 0;
  linePath.value = "";
}

function closeManualShortcutEditor() {
  manualEditor.value = null;
  void nextTick().then(scheduleUpdateLine);
}

function applyManualShortcut(keys: number[]) {
  const editor = manualEditor.value;
  if (!editor) return;
  manualEditor.value = null;
  applyCapturedKeys(editor.buttonId, keys);
  void nextTick().then(scheduleUpdateLine);
}

const stageRef = ref<HTMLElement | null>(null);
const remoteRef = ref<InstanceType<typeof RemoteHotspot> | null>(null);
const cardRefs = ref<Record<string, HTMLElement | null>>({});

const linePath = ref("");
const lineOpacity = ref(0);
const lineStrong = ref(true);
const dotA = ref({ x: 0, y: 0 });
const dotB = ref({ x: 0, y: 0 });
const svgSize = ref({ w: 0, h: 0 });

let unlistenCaptured: UnlistenFn | null = null;
let unlistenProgress: UnlistenFn | null = null;
let pollTimer: ReturnType<typeof setInterval> | null = null;
let applied = false;
let resizeObs: ResizeObserver | null = null;
let lineRaf: number | null = null;
let micFlashTimer: ReturnType<typeof setTimeout> | null = null;

const micBindFlash = ref(false);

function actionToVks(action: KeyAction): number[] | null {
  if (!action || action.type === "None") return null;
  if (action.type === "SingleKey") return [Number(action.value)];
  if (action.type === "ComboKey" && Array.isArray(action.value)) {
    return action.value.map((v) => Number(v));
  }
  return null;
}

function triggerMicBindFlash() {
  micBindFlash.value = false;
  void nextTick(() => {
    if (micFlashTimer) clearTimeout(micFlashTimer);
    micBindFlash.value = true;
    micFlashTimer = setTimeout(() => {
      micBindFlash.value = false;
      micFlashTimer = null;
    }, 1200);
  });
}

function micBindingSignature(): string {
  const mic = actionOf("mic");
  const vks = actionToVks(mic);
  return JSON.stringify({
    vks,
    mode: props.config.trigger_mode,
    release: props.config.voice_release_behavior,
  });
}

let lastMicBindingSig = micBindingSignature();

watch(
  () => micBindingSignature(),
  (sig) => {
    if (sig === lastMicBindingSig) return;
    lastMicBindingSig = sig;
    triggerMicBindFlash();
  },
);

function setCardRef(id: string, el: unknown) {
  cardRefs.value[id] = (el as HTMLElement) || null;
}

function labelOf(id: string): string {
  return props.config.button_aliases?.[id] || DEFAULT_LABELS[id] || id;
}

function actionOf(id: string): KeyAction {
  return (
    props.config.button_bindings?.[id] || { type: "None", value: null }
  );
}

function actionLabel(action: KeyAction): string {
  if (!action || action.type === "None") return "未绑定";
  if (action.type === "SingleKey") return vkDisplayName(Number(action.value));
  if (action.type === "ComboKey") {
    const arr = Array.isArray(action.value) ? action.value : [];
    return arr.map((v) => vkDisplayName(Number(v))).join(" + ");
  }
  if (action.type === "TextInput") return `文字: ${action.value}`;
  if (action.type === "LaunchApp") return `启动: ${action.value}`;
  return "—";
}

function pickMediaKey(vk: number) {
  if (!capturing.value) return;
  onCaptured([vk], [vkDisplayName(vk)]);
}

function vksToHotkeyNames(vks: number[]): string[] {
  const map: Record<number, string> = {
    0xa2: "leftctrl",
    0xa3: "rightctrl",
    0x11: "ctrl",
    0xa0: "leftshift",
    0xa1: "rightshift",
    0x10: "shift",
    0xa4: "leftalt",
    0xa5: "rightalt",
    0x12: "alt",
    0x5b: "leftwin",
    0x5c: "rightwin",
    0x20: "space",
    0x0d: "enter",
  };
  return vks.map((vk) => {
    if (map[vk]) return map[vk];
    if (vk >= 0x41 && vk <= 0x5a) return String.fromCharCode(vk).toLowerCase();
    if (vk >= 0x30 && vk <= 0x39) return String(vk - 0x30);
    if (vk >= 0x70 && vk <= 0x7b) return `f${vk - 0x6f}`;
    return `vk_${vk.toString(16)}`;
  });
}

type RightCard = {
  id: string;
  label: string;
  action: ReturnType<typeof actionOf>;
  side: "right";
  ghost: boolean;
};

const leftButtons = computed(() =>
  LEFT_IDS.map((id) => ({
    id,
    label: labelOf(id),
    action: actionOf(id),
    side: "left" as const,
  }))
);

/** 右列：语音后插入空槽（对齐 V4.2，保持左右等距） */
const rightButtons = computed<RightCard[]>(() => {
  const cards: RightCard[] = RIGHT_IDS.map((id) => ({
    id,
    label: labelOf(id),
    action: actionOf(id),
    side: "right",
    ghost: false,
  }));
  const micIdx = cards.findIndex((c) => c.id === "mic");
  if (micIdx >= 0) {
    cards.splice(micIdx + 1, 0, {
      id: "_spacer",
      label: "",
      action: actionOf("tv"),
      side: "right",
      ghost: true,
    });
  }
  return cards;
});

const activeLineId = computed(
  () => selectedId.value || hoverId.value || pressedId.value || null
);

function edgeToward(
  el: HTMLElement,
  stageBox: DOMRect,
  side: "left" | "right"
) {
  const r = el.getBoundingClientRect();
  const y = r.top + r.height / 2 - stageBox.top;
  // left 侧卡片：取右边缘；right 侧卡片：取左边缘
  if (side === "left") {
    return { x: r.right - stageBox.left, y };
  }
  return { x: r.left - stageBox.left, y };
}

/** 按键锚点：朝映射块一侧的边缘中点，避免线穿过键帽文字 */
function keyEdgeToward(
  el: HTMLElement,
  stageBox: DOMRect,
  side: "left" | "right"
) {
  const r = el.getBoundingClientRect();
  const y = r.top + r.height / 2 - stageBox.top;
  // 左栏连线接到按键左缘；右栏接到按键右缘
  if (side === "left") {
    return { x: r.left - stageBox.left, y };
  }
  return { x: r.right - stageBox.left, y };
}

/** 合并到下一帧，避免 ResizeObserver ↔ 改 SVG 尺寸反馈环把 WebView 卡死 */
function scheduleUpdateLine() {
  if (lineRaf != null) return;
  lineRaf = requestAnimationFrame(() => {
    lineRaf = null;
    updateLine();
  });
}

function updateLine() {
  const id = activeLineId.value;
  const stage = stageRef.value;
  // composer 打开：一律隐藏连线，避免回流期端点乱跳（对齐 v5.0）
  if (manualEditor.value || !id || !stage) {
    if (lineOpacity.value !== 0) lineOpacity.value = 0;
    if (linePath.value) linePath.value = "";
    return;
  }

  const stageBox = stage.getBoundingClientRect();
  const w = Math.round(stageBox.width);
  const h = Math.round(stageBox.height);
  // 仅改 viewBox 坐标；SVG 用 CSS 铺满，禁止写 width/height 属性（易触发滚动条抖动环）
  if (svgSize.value.w !== w || svgSize.value.h !== h) {
    svgSize.value = { w, h };
  }

  const card = cardRefs.value[id];
  const key = remoteRef.value?.keyEl?.(id) as HTMLElement | null;
  if (!card || !key) {
    if (lineOpacity.value !== 0) lineOpacity.value = 0;
    if (linePath.value) linePath.value = "";
    return;
  }

  const side = (LEFT_IDS as readonly string[]).includes(id) ? "left" : "right";
  const keyPt = keyEdgeToward(key, stageBox, side);
  const cardPt = edgeToward(card, stageBox, side);

  const dx = Math.max(40, Math.abs(keyPt.x - cardPt.x) * 0.45);
  const c1 =
    side === "left"
      ? { x: cardPt.x + dx, y: cardPt.y }
      : { x: cardPt.x - dx, y: cardPt.y };
  const c2 =
    side === "left"
      ? { x: keyPt.x - dx * 0.25, y: keyPt.y }
      : { x: keyPt.x + dx * 0.25, y: keyPt.y };

  const nextPath = `M ${cardPt.x} ${cardPt.y} C ${c1.x} ${c1.y}, ${c2.x} ${c2.y}, ${keyPt.x} ${keyPt.y}`;
  const strong = selectedId.value === id;
  const opacity = strong ? 1 : 0.45;
  if (linePath.value !== nextPath) linePath.value = nextPath;
  if (dotA.value.x !== cardPt.x || dotA.value.y !== cardPt.y) dotA.value = cardPt;
  if (dotB.value.x !== keyPt.x || dotB.value.y !== keyPt.y) dotB.value = keyPt;
  if (lineStrong.value !== strong) lineStrong.value = strong;
  if (lineOpacity.value !== opacity) lineOpacity.value = opacity;
}

async function selectButton(id: string) {
  if (selectedId.value === id) {
    if (capturing.value) {
      await cancelCapture();
    }
    selectedId.value = null;
    captureError.value = null;
    await nextTick();
    updateLine();
    return;
  }
  if (capturing.value) {
    await cancelCapture();
  }
  selectedId.value = id;
  captureError.value = null;
  await nextTick();
  updateLine();
}

function onRemoteHover(id: string | null) {
  hoverId.value = id;
  updateLine();
}

function onCardHover(id: string | null) {
  hoverId.value = id;
  updateLine();
}

/** 实体键与遥控器同步：down 亮、按住保持、up 灭；漏 up 时兜底清除 */
function clearPressed(id?: string) {
  if (pressClearTimer) {
    clearTimeout(pressClearTimer);
    pressClearTimer = null;
  }
  if (id && pressedId.value && pressedId.value !== id) return;
  pressedId.value = null;
  updateLine();
}

watch(
  () => props.pressPulse,
  (p) => {
    if (!p?.id) return;
    const id = p.id === "voice" ? "mic" : p.id;
    if (p.phase === "up") {
      clearPressed(id);
      return;
    }
    pressedId.value = id;
    updateLine();
    // 兜底：丢 up 事件时不一直亮
    if (pressClearTimer) clearTimeout(pressClearTimer);
    pressClearTimer = setTimeout(() => {
      clearPressed(id);
    }, 15000);
  },
);

onUnmounted(() => {
  if (pressClearTimer) clearTimeout(pressClearTimer);
});

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

/** 录入期间：优先用 WebView keydown 直接录入（不依赖 LL 钩子收到键） */
function vkFromEvent(e: KeyboardEvent): number {
  return e.keyCode || e.which || 0;
}
function isModVk(vk: number): boolean {
  return (
    vk === 0x10 ||
    vk === 0x11 ||
    vk === 0x12 ||
    vk === 0x5b ||
    vk === 0x5c ||
    vk === 0xa0 ||
    vk === 0xa1 ||
    vk === 0xa2 ||
    vk === 0xa3 ||
    vk === 0xa4 ||
    vk === 0xa5
  );
}
function chordFromEvent(e: KeyboardEvent): number[] {
  const main = vkFromEvent(e);
  if (!main || isModVk(main)) return [];
  const mods: number[] = [];
  if (e.ctrlKey) mods.push(e.location === 2 ? 0xa3 : 0xa2);
  if (e.shiftKey) mods.push(0xa0);
  if (e.altKey) mods.push(e.location === 2 ? 0xa5 : 0xa4);
  if (e.metaKey) mods.push(0x5b);
  return [...mods, main];
}

function blockBrowserKeysDuringCapture(e: KeyboardEvent) {
  if (!capturing.value) return;
  e.preventDefault();
  e.stopPropagation();
  if (applied) return;
  const chord = chordFromEvent(e);
  if (chord.length > 0) {
    // 键已到 WebView = 至少能从前端录；不依赖钩子是否吞住
    const labels = chord.map((vk) => vkDisplayName(vk));
    void onCaptured(chord, labels);
  }
}

function startPolling() {
  stopPolling();
  applied = false;
  pollTimer = setInterval(async () => {
    if (!capturing.value || applied) return;
    try {
      // 进度也走 IPC：部分机器上 emit("shortcut-capture-progress") 会丢/延迟，
      // 仅靠 listen 会出现「按着没反应，松手后其实已录入」。
      const snap = await invoke<{
        pending: { keys: number[]; labels: string[] } | null;
        progress: string[];
        leaks?: number;
        healthFailed?: boolean;
      }>("capture_shortcut_poll");
      // healthFailed 只表示钩子没吞住原生键；WebView 仍可录入，不中止。
      if (Array.isArray(snap?.progress) && snap.progress.length > 0) {
        liveLabels.value = snap.progress;
      }
      const result = snap?.pending;
      if (result && Array.isArray(result.keys) && result.keys.length > 0) {
        onCaptured(result.keys, result.labels || []);
      }
    } catch (e) {
      console.warn("capture poll failed", e);
    }
  }, 50);
}

async function onCaptured(keys: number[], labels: string[]) {
  if (applied) return;
  applied = true;
  stopPolling();
  liveLabels.value = [];

  const buttonId = selectedId.value;
  if (buttonId && keys?.length) {
    applyCapturedKeys(buttonId, keys);
  }
  try {
    await invoke("capture_shortcut_stop");
  } catch {
    /* ignore */
  }
  capturing.value = false;
  void nextTick().then(updateLine);
}

async function startCapture() {
  const buttonId = selectedId.value;
  if (!buttonId) return;
  if (capturing.value) {
    await cancelCapture();
    return;
  }
  captureError.value = null;
  liveLabels.value = [];
  applied = false;
  // 后端探针通过后才进入 capturing（禁止先亮 UI）
  try {
    await invoke("capture_shortcut_start");
    capturing.value = true;
    startPolling();
  } catch (e) {
    capturing.value = false;
    stopPolling();
    captureError.value = String(e);
  }
}

async function cancelCapture() {
  stopPolling();
  capturing.value = false;
  liveLabels.value = [];
  applied = false;
  try {
    await invoke("capture_shortcut_stop");
  } catch {
    /* ignore */
  }
}

function applyCapturedKeys(buttonId: string, vks: number[]) {
  let action: KeyAction;
  if (!vks.length) {
    action = { type: "None", value: null };
  } else if (vks.length === 1) {
    action = { type: "SingleKey", value: vks[0] };
  } else {
    action = { type: "ComboKey", value: [...vks] };
  }
  if (!props.config.button_bindings) {
    (props.config as DeviceConfig).button_bindings = {};
  }
  props.config.button_bindings[buttonId] = action;
  const next: DeviceConfig = {
    ...props.config,
    button_bindings: { ...props.config.button_bindings },
  };
  if (buttonId === "mic" || buttonId === "voice") {
    next.button_bindings.mic = action;
    next.button_bindings.voice = action;
    next.voice_hotkey = vksToHotkeyNames(vks);
  }
  emit("save", next);
}

function clearBinding(buttonId: string) {
  props.config.button_bindings[buttonId] = { type: "None", value: null };
  const next: DeviceConfig = {
    ...props.config,
    button_bindings: { ...props.config.button_bindings },
  };
  if (buttonId === "mic" || buttonId === "voice") {
    next.button_bindings.mic = { type: "None", value: null };
    next.button_bindings.voice = { type: "None", value: null };
    next.voice_hotkey = [];
  }
  emit("save", next);
}

watch([selectedId, hoverId, manualEditor], () => {
  void nextTick().then(scheduleUpdateLine);
});

onMounted(async () => {
  try {
    unlistenCaptured = await listen<{ keys: number[]; labels: string[] }>(
      "shortcut-captured",
      (event) => {
        const keys = event.payload?.keys;
        if (!keys?.length) return;
        onCaptured(keys, event.payload.labels || []);
      }
    );
    unlistenProgress = await listen<{ labels: string[] }>(
      "shortcut-capture-progress",
      (event) => {
        liveLabels.value = event.payload?.labels || [];
      }
    );
  } catch (e) {
    console.warn("shortcut listen failed", e);
  }

  if (stageRef.value) {
    resizeObs = new ResizeObserver(() => scheduleUpdateLine());
    resizeObs.observe(stageRef.value);
  }
  stageRef.value?.addEventListener("scroll", scheduleUpdateLine, { passive: true });
  window.addEventListener("resize", scheduleUpdateLine);
  window.addEventListener("keydown", blockBrowserKeysDuringCapture, true);
  window.addEventListener("keyup", blockBrowserKeysDuringCapture, true);
});

onUnmounted(() => {
  stopPolling();
  unlistenCaptured?.();
  unlistenProgress?.();
  resizeObs?.disconnect();
  if (lineRaf != null) {
    cancelAnimationFrame(lineRaf);
    lineRaf = null;
  }
  if (micFlashTimer) {
    clearTimeout(micFlashTimer);
    micFlashTimer = null;
  }
  stageRef.value?.removeEventListener("scroll", scheduleUpdateLine);
  window.removeEventListener("resize", scheduleUpdateLine);
  window.removeEventListener("keydown", blockBrowserKeysDuringCapture, true);
  window.removeEventListener("keyup", blockBrowserKeysDuringCapture, true);
  if (capturing.value) {
    invoke("capture_shortcut_stop").catch(() => {});
  }
});
</script>

<template>
  <div class="stage-scroll">
    <div ref="stageRef" class="mapping-stage">
      <svg
        class="line-layer"
        :viewBox="`0 0 ${svgSize.w || 1} ${svgSize.h || 1}`"
        aria-hidden="true"
      >
        <path
          v-if="linePath"
          :d="linePath"
          fill="none"
          :stroke="lineStrong ? '#2563eb' : '#94a3b8'"
          :stroke-width="lineStrong ? 2.2 : 1.5"
          stroke-linecap="round"
          :opacity="lineOpacity"
        />
        <circle
          v-if="linePath"
          :cx="dotA.x"
          :cy="dotA.y"
          r="3.5"
          :fill="lineStrong ? '#2563eb' : '#94a3b8'"
          :opacity="lineOpacity"
        />
        <circle
          v-if="linePath"
          :cx="dotB.x"
          :cy="dotB.y"
          r="3.5"
          :fill="lineStrong ? '#2563eb' : '#94a3b8'"
          :opacity="lineOpacity"
        />
      </svg>

      <aside class="side-col left-col">
        <VoiceShortcutComposer
          v-if="manualEditor && isLeftButton(manualEditor.buttonId)"
          :initial-keys="manualEditor.initialKeys"
          :button-label="DEFAULT_LABELS[manualEditor.buttonId] || manualEditor.buttonId"
          slot-label="快捷键"
          @apply="applyManualShortcut"
          @cancel="closeManualShortcutEditor"
        />
        <div
          v-for="btn in leftButtons"
          :key="btn.id"
          :ref="(el) => setCardRef(btn.id, el)"
          class="map-card"
          :class="{
            active: selectedId === btn.id,
            hover:
              (hoverId === btn.id || pressedId === btn.id) &&
              selectedId !== btn.id,
            pressed: pressedId === btn.id,
          }"
          @mouseenter="onCardHover(btn.id)"
          @mouseleave="onCardHover(null)"
          @click="selectButton(btn.id)"
        >
          <div class="map-card-main">
            <span class="map-name">
              <RemoteKeyIcon :key-id="btn.id" />
              {{ btn.label }}
            </span>
            <span
              :class="['map-bind', { unbound: btn.action.type === 'None' }]"
            >
              {{ actionLabel(btn.action) }}
            </span>
          </div>
          <div class="map-card-actions" @click.stop>
            <div class="map-card-actions-inner">
              <button
                type="button"
                class="btn-sm btn-edit"
                :disabled="capturing && selectedId !== btn.id"
                @click="startCapture"
              >
                {{ capturing && selectedId === btn.id ? "取消录入" : "录入" }}
              </button>
              <button
                type="button"
                class="btn-sm btn-edit"
                @click.stop="openManualShortcutEditor"
              >手动组合</button>
              <button
                v-if="btn.action.type !== 'None'"
                type="button"
                class="btn-sm btn-clear"
                :disabled="capturing"
                @click="clearBinding(btn.id)"
              >
                清除
              </button>
              <p
                v-if="capturing && selectedId === btn.id"
                class="capture-live"
                :class="{ 'capture-hint-blink': !liveLabels.length }"
              >
                {{
                  liveLabels.length
                    ? liveLabels.join(" + ") + " …"
                    : "请按目标键或组合键"
                }}
              </p>
              <div
                v-if="capturing && selectedId === btn.id"
                class="media-pick"
              >
                <span class="media-pick-label">设置为：</span>
                <button
                  v-for="k in MEDIA_PICK_KEYS"
                  :key="k.vk"
                  type="button"
                  class="btn-sm btn-media"
                  @click="pickMediaKey(k.vk)"
                >
                  {{ k.label }}
                </button>
              </div>
              <p v-if="captureError && selectedId === btn.id" class="capture-err">
                {{ captureError }}
              </p>
            </div>
          </div>
        </div>
      </aside>

      <div class="center-stage">
        <RemoteHotspot
          ref="remoteRef"
          :selected-id="selectedId"
          :hover-id="pressedId || hoverId"
          @select="selectButton"
          @hover="onRemoteHover"
        />
      </div>

      <aside class="side-col right-col">
        <VoiceShortcutComposer
          v-if="manualEditor && !isLeftButton(manualEditor.buttonId)"
          :initial-keys="manualEditor.initialKeys"
          :button-label="DEFAULT_LABELS[manualEditor.buttonId] || manualEditor.buttonId"
          slot-label="快捷键"
          @apply="applyManualShortcut"
          @cancel="closeManualShortcutEditor"
        />
        <template v-for="btn in rightButtons" :key="btn.id">
          <div
            v-if="btn.ghost"
            class="map-card-spacer"
            aria-hidden="true"
          />
          <div
            v-else
            :ref="(el) => setCardRef(btn.id, el)"
            class="map-card"
            :class="{
              active: selectedId === btn.id,
              hover:
                (hoverId === btn.id || pressedId === btn.id) &&
                selectedId !== btn.id,
              pressed: pressedId === btn.id,
            }"
            @mouseenter="onCardHover(btn.id)"
            @mouseleave="onCardHover(null)"
            @click="selectButton(btn.id)"
          >
            <div class="map-card-main">
              <span class="map-name">
                <RemoteKeyIcon :key-id="btn.id" />
                {{ btn.label }}
              </span>
              <span
                :class="[
                  'map-bind',
                  {
                    unbound: btn.action.type === 'None',
                    'mic-bind-flash': btn.id === 'mic' && micBindFlash,
                  },
                ]"
              >
                {{ actionLabel(btn.action) }}
              </span>
            </div>
            <div class="map-card-actions" @click.stop>
              <div class="map-card-actions-inner">
                <button
                  type="button"
                  class="btn-sm btn-edit"
                  :disabled="capturing && selectedId !== btn.id"
                  @click="startCapture"
                >
                  {{ capturing && selectedId === btn.id ? "取消录入" : "录入" }}
                </button>
                <button
                  type="button"
                  class="btn-sm btn-edit"
                  @click.stop="openManualShortcutEditor"
                >手动组合</button>
                <button
                  v-if="btn.action.type !== 'None'"
                  type="button"
                  class="btn-sm btn-clear"
                  :disabled="capturing"
                  @click="clearBinding(btn.id)"
                >
                  清除
                </button>
                <p
                  v-if="capturing && selectedId === btn.id"
                  class="capture-live"
                  :class="{ 'capture-hint-blink': !liveLabels.length }"
                >
                  {{
                    liveLabels.length
                      ? liveLabels.join(" + ") + " …"
                      : "请按目标键或组合键"
                  }}
                </p>
                <div
                  v-if="capturing && selectedId === btn.id"
                  class="media-pick"
                >
                  <span class="media-pick-label">设置为：</span>
                  <button
                    v-for="k in MEDIA_PICK_KEYS"
                    :key="k.vk"
                    type="button"
                    class="btn-sm btn-media"
                    @click="pickMediaKey(k.vk)"
                  >
                    {{ k.label }}
                  </button>
                </div>
                <p v-if="captureError && selectedId === btn.id" class="capture-err">
                  {{ captureError }}
                </p>
              </div>
            </div>
          </div>
        </template>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.stage-scroll {
  overflow-x: hidden;
  margin: 0 -4px;
  padding-bottom: 4px;
}

.mapping-stage {
  position: relative;
  display: grid;
  grid-template-columns: 224px auto 224px;
  gap: 10px 14px;
  align-items: start;
  justify-content: center;
  max-width: 720px;
  margin: 0 auto;
  min-width: 0;
  width: 100%;
  padding: 4px 0 8px;
  box-sizing: border-box;
}

.line-layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 5;
  overflow: visible;
}

.side-col {
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 2;
  min-width: 0;
  width: 100%;
  max-width: 224px;
  padding-top: 0;
  /* 溢出只允许纵向，子项不压扁（对齐 v5.0 灰线修复） */
  overflow-x: hidden;
  overflow-y: auto;
  max-height: var(--col-max, 480px);
  overscroll-behavior: contain;
  scrollbar-width: thin;
  scrollbar-color: #3a424e transparent;
}

.side-col > * {
  flex-shrink: 0;
}

.side-col::-webkit-scrollbar {
  width: 6px;
  height: 0;
}

.side-col::-webkit-scrollbar-track {
  background: transparent;
}

.side-col::-webkit-scrollbar-thumb {
  background: #3a424e;
  border-radius: 99px;
}

.left-col {
  align-items: stretch;
}

.right-col {
  align-items: stretch;
}

.center-stage {
  z-index: 2;
  justify-self: center;
  align-self: start;
  padding: 0;
  margin: 0;
  background: transparent;
  border: none;
  box-shadow: none;
}

.map-card {
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 10px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s, background 0.15s;
  min-width: 0;
  width: 100%;
  max-width: 224px;
  box-sizing: border-box;
  flex-shrink: 0;
}

/* 右列空槽：同尺寸虚线框，不可点（V4.2） */
.map-card-spacer {
  width: 100%;
  max-width: 224px;
  min-height: 38px;
  box-sizing: border-box;
  flex: 0 0 auto;
  border: 1px dashed rgba(52, 59, 70, 0.9);
  border-radius: 8px;
  background: transparent;
  pointer-events: none;
}

.map-card:hover,
.map-card.hover {
  border-color: var(--primary);
  background: var(--surface-hover);
}

.map-card.pressed {
  border-color: var(--primary);
  background: var(--surface-selected);
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.18);
}

.map-card.active {
  border-color: var(--primary);
  box-shadow: 0 0 0 2px rgba(37, 99, 235, 0.15);
  background: var(--surface-selected);
}

.map-card-main {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-width: 0;
}

.map-name {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 400;
  color: var(--text);
  flex-shrink: 1;
  min-width: 0;
  overflow: hidden;
}

.map-bind {
  font-size: 11px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  background: var(--panel-2);
  color: var(--text);
  padding: 2px 8px;
  border-radius: 4px;
  min-width: 0;
  max-width: 72%;
  flex: 0 1 auto;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: right;
}

.map-bind.unbound {
  background: transparent;
  color: #94a3b8;
  border: 1px dashed #3a424e;
  padding: 1px 7px;
  border-radius: 4px;
}

@keyframes mic-bind-flash {
  0%,
  100% {
    background: var(--panel-2);
    color: var(--text);
    box-shadow: none;
  }
  33% {
    background: #dbeafe;
    color: #1d4ed8;
    box-shadow: 0 0 0 2px rgba(37, 99, 235, 0.22);
  }
  66% {
    background: var(--surface-selected);
    color: var(--primary);
    box-shadow: 0 0 0 1px rgba(37, 99, 235, 0.18);
  }
}

.map-bind.mic-bind-flash:not(.unbound) {
  animation: mic-bind-flash 0.38s ease-in-out 3;
}

.btn-sm {
  padding: 4px 10px;
  border: 1px solid var(--edge);
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  background: var(--panel-2);
  color: var(--text);
  transition: transform 160ms cubic-bezier(0.23, 1, 0.32, 1),
    background-color 150ms cubic-bezier(0.23, 1, 0.32, 1),
    border-color 150ms cubic-bezier(0.23, 1, 0.32, 1),
    color 150ms cubic-bezier(0.23, 1, 0.32, 1);
}
.btn-sm:active:not(:disabled) {
  transform: scale(0.97);
}

.btn-edit {
  color: var(--primary);
  border-color: #2f6a5f;
}
.btn-edit:hover:not(:disabled) {
  background: var(--surface-selected);
}
.btn-clear {
  color: var(--danger);
  border-color: #6a3a3a;
}
.btn-clear:hover:not(:disabled) {
  background: #3a2424;
  border-color: var(--danger);
}
.btn-sm:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.map-card-actions {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 180ms cubic-bezier(0.23, 1, 0.32, 1);
}
.map-card.active .map-card-actions {
  grid-template-rows: 1fr;
}
.map-card-actions-inner {
  overflow: hidden;
  min-height: 0;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  padding: 0;
  border-top: 1px solid transparent;
  transition: padding 180ms cubic-bezier(0.23, 1, 0.32, 1),
    border-color 180ms cubic-bezier(0.23, 1, 0.32, 1),
    opacity 150ms cubic-bezier(0.23, 1, 0.32, 1);
  opacity: 0;
}
.map-card.active .map-card-actions-inner {
  padding: 8px 0;
  border-top-color: var(--border);
  opacity: 1;
}

.capture-live {
  width: 100%;
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--primary);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}

.capture-live.capture-hint-blink {
  text-align: center;
  color: #ea580c;
  font-weight: 600;
  animation: capture-hint-blink 1s ease-in-out infinite;
}

@keyframes capture-hint-blink {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.3;
  }
}

.media-pick {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  width: 100%;
  margin-top: 4px;
}

.media-pick-label {
  font-size: 12px;
  color: var(--text-secondary);
  flex-shrink: 0;
}

.btn-media {
  color: var(--primary);
  border-color: #2f6a5f;
  background: #1a2c29;
  padding: 3px 8px;
}
.btn-media:hover:not(:disabled) {
  background: #1e3d38;
}

.capture-err {
  width: 100%;
  margin: 2px 0 0;
  font-size: 12px;
  color: var(--danger);
}

@media (prefers-reduced-motion: reduce) {
  .btn-sm,
  .btn-sm:active:not(:disabled),
  .map-card-actions,
  .map-card-actions-inner,
  .capture-live.capture-hint-blink,
  .map-card,
  .key-cap {
    transition: none !important;
    animation: none !important;
    transform: none !important;
  }
  .map-card.active .map-card-actions-inner {
    opacity: 1;
  }
  .map-card-actions-inner {
    opacity: 1;
  }
}
</style>
