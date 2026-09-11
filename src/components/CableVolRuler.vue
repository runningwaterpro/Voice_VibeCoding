<script setup lang="ts">
import { computed } from "vue";
import {
  CABLE_VOL_DB_HIGH,
  CABLE_VOL_DB_LOW,
  CABLE_VOL_DB_MIN,
  CABLE_VOL_DB_MAX,
  CABLE_VOL_TICKS,
  cableDbToPct,
  cableLevelToDb,
  cableZoneForDb,
} from "../utils/cableVolMeter";

const props = withDefaults(
  defineProps<{
    level: number;
    disabled?: boolean;
    active?: boolean;
    label?: string;
    /** 右侧短提示，与标尺同行 */
    hint?: string;
  }>(),
  {
    label: "电平",
    hint: "",
  }
);

const db = computed(() => cableLevelToDb(props.level));
const markerPct = computed(() => cableDbToPct(db.value));
const zone = computed(() =>
  props.disabled ? "idle" : cableZoneForDb(db.value)
);
const dbText = computed(() => {
  if (props.disabled || !props.active) return "—";
  return `${Math.round(db.value)}`;
});
const clipped = computed(() => props.active && props.level >= 0.995);

const shellState = computed(() => {
  if (props.disabled) return "disabled";
  if (props.active) return "active";
  return "idle";
});

const zoneLowWidth = cableDbToPct(CABLE_VOL_DB_LOW);
const zoneOkWidth = cableDbToPct(CABLE_VOL_DB_HIGH) - zoneLowWidth;
const zoneHighWidth = 100 - cableDbToPct(CABLE_VOL_DB_HIGH);

const ariaLabel = computed(() => {
  if (props.disabled) return `${props.label}标尺（未就绪）`;
  if (!props.active) return `${props.label}标尺：无信号`;
  return `${props.label} ${Math.round(db.value)} dBFS${props.hint ? ` ${props.hint}` : ""}`;
});
</script>

<template>
  <div
    class="cable-vol-ruler"
    :class="[`zone-${zone}`, `shell-${shellState}`]"
    role="meter"
    :aria-valuenow="Math.round(db)"
    :aria-valuemin="CABLE_VOL_DB_MIN"
    :aria-valuemax="CABLE_VOL_DB_MAX"
    :aria-label="ariaLabel"
  >
    <span class="ruler-caption">{{ label }}</span>
    <div class="ruler-track">
      <div class="ruler-zones" aria-hidden="true">
        <span class="ruler-zone zone-low" :style="{ width: `${zoneLowWidth}%` }" />
        <span class="ruler-zone zone-ok" :style="{ width: `${zoneOkWidth}%` }" />
        <span class="ruler-zone zone-high" :style="{ width: `${zoneHighWidth}%` }" />
      </div>
      <span
        v-for="tick in CABLE_VOL_TICKS"
        :key="tick"
        class="ruler-tick"
        :style="{ left: `${cableDbToPct(tick)}%` }"
        aria-hidden="true"
      />
      <span
        class="ruler-marker"
        :style="{ left: `${markerPct}%` }"
        aria-hidden="true"
      />
    </div>
    <span v-if="hint" class="ruler-hint" :class="`is-${zone}`">{{ hint }}</span>
    <span class="ruler-db" :class="{ 'is-clipped': clipped }">{{ dbText }}</span>
  </div>
</template>

<style scoped>
.cable-vol-ruler {
  width: 100%;
  height: 22px;
  padding: 0 4px;
  box-sizing: border-box;
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 8px;
  border-radius: 4px;
  background: #f1f5f9;
  border: 1px solid var(--border, #e2e8f0);
  overflow: hidden;
}

.cable-vol-ruler.shell-active {
  background: #ecfdf5;
  border-color: #bbf7d0;
}

.ruler-caption {
  flex: 0 0 auto;
  font-size: 11px;
  line-height: 1;
  color: #64748b;
  white-space: nowrap;
}

.ruler-track {
  position: relative;
  flex: 1 1 auto;
  min-width: 48px;
  height: 8px;
  border-radius: 3px;
  border: 1px solid transparent;
  background: transparent;
}

.cable-vol-ruler.shell-idle .ruler-zone,
.cable-vol-ruler.shell-disabled .ruler-zone {
  opacity: 0;
}

.cable-vol-ruler.shell-idle .ruler-track,
.cable-vol-ruler.shell-disabled .ruler-track {
  border-color: transparent;
  background: transparent;
}

.cable-vol-ruler.shell-active .ruler-track {
  border-color: rgba(187, 247, 208, 0.9);
  background: rgba(255, 255, 255, 0.35);
}

.ruler-zones {
  position: absolute;
  inset: 0;
  display: flex;
  border-radius: 3px;
  overflow: hidden;
}

.ruler-zone {
  height: 100%;
  opacity: 0.35;
}

.zone-low {
  background: #fde68a;
}

.zone-ok {
  background: #bbf7d0;
}

.zone-high {
  background: #fecaca;
}

.ruler-tick {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  margin-left: -0.5px;
  background: rgba(100, 116, 139, 0.35);
  pointer-events: none;
}

.ruler-marker {
  position: absolute;
  top: -2px;
  bottom: -2px;
  width: 2px;
  margin-left: -1px;
  border-radius: 1px;
  background: #94a3b8;
  box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.85);
  transition: left 70ms linear, background-color 120ms ease;
  pointer-events: none;
}

.shell-active.zone-low .ruler-marker {
  background: #ca8a04;
}

.shell-active.zone-ok .ruler-marker {
  background: #16a34a;
}

.shell-active.zone-high .ruler-marker {
  background: #dc2626;
}

.ruler-hint {
  flex: 0 1 auto;
  font-size: 11px;
  line-height: 1;
  color: #64748b;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 42%;
}

.ruler-hint.is-low {
  color: #a16207;
}

.ruler-hint.is-ok {
  color: #15803d;
}

.ruler-hint.is-high {
  color: #dc2626;
}

.ruler-db {
  flex: 0 0 auto;
  min-width: 2.2em;
  text-align: right;
  font-size: 11px;
  line-height: 1;
  color: #64748b;
  font-variant-numeric: tabular-nums;
}

.ruler-db.is-clipped {
  color: #dc2626;
  font-weight: 600;
}
</style>
