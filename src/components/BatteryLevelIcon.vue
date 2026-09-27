<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  level: number | null;
}>();

const clampedLevel = computed(() => {
  if (props.level == null || Number.isNaN(props.level)) return null;
  return Math.max(0, Math.min(100, Math.round(props.level)));
});

/* 电量读数会频繁变化，用 scaleX 走合成层，避免 width 触发布局 */
const fillTransform = computed(() => {
  const level = clampedLevel.value;
  return `scaleX(${level == null ? 0 : level / 100})`;
});

const tone = computed(() => {
  const level = clampedLevel.value;
  if (level == null) return "unknown";
  if (level < 10) return "red";
  if (level < 25) return "yellow";
  return "green";
});

const ariaLabel = computed(() => {
  if (clampedLevel.value == null) return "电量未知";
  return `电量 ${clampedLevel.value}%`;
});
</script>

<template>
  <span
    class="battery-icon"
    :class="`is-${tone}`"
    role="img"
    :aria-label="ariaLabel"
  >
    <span class="battery-body">
      <span class="battery-track">
        <span class="battery-fill" :style="{ transform: fillTransform }" />
      </span>
    </span>
    <span class="battery-cap" aria-hidden="true" />
  </span>
</template>

<style scoped>
.battery-icon {
  display: inline-flex;
  align-items: center;
  gap: 1px;
  flex-shrink: 0;
  color: #94a3b8;
}

.battery-body {
  box-sizing: border-box;
  position: relative;
  width: 24px;
  height: 12px;
  border: 1px solid currentColor;
  border-radius: 2px;
  flex-shrink: 0;
}

/* 内缩会让 100% 时填充条仍差 1px 碰不到边框，看起来“不满”。
   轨道贴住 padding box，边框本身承担外轮廓，scaleX(1) 才是真正的满格。 */
.battery-track {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  border-radius: 1px;
  overflow: hidden;
}

/* 圆角交给 .battery-track 的 overflow 裁剪：scaleX 会把圆角横向拉变形 */
.battery-fill {
  display: block;
  width: 100%;
  height: 100%;
  transform-origin: left center;
  background: currentColor;
  transition: transform 0.25s ease, background-color 0.25s ease;
}

@media (prefers-reduced-motion: reduce) {
  .battery-fill {
    transition: none;
  }
}

.battery-cap {
  width: 2px;
  height: 5px;
  border-radius: 0 1px 1px 0;
  background: currentColor;
}

.battery-icon.is-green {
  color: #16a34a;
}

.battery-icon.is-yellow {
  color: #ca8a04;
}

.battery-icon.is-red {
  color: #dc2626;
}

.battery-icon.is-unknown {
  color: #94a3b8;
}
</style>
