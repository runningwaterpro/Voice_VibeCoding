/** 送声/输入标尺：电平 0..1 → dBFS（RMS 为主，量程 -60..0） */

export const CABLE_VOL_DB_MIN = -60;
export const CABLE_VOL_DB_MAX = 0;
export const CABLE_VOL_DB_LOW = -36;
export const CABLE_VOL_DB_HIGH = -6;

export const CABLE_VOL_TICKS = [-48, -36, -24, -12, 0] as const;

export function cableLevelToDb(level: number): number {
  const v = Math.max(0, Math.min(1, level));
  if (v < 0.00025) return CABLE_VOL_DB_MIN;
  return Math.max(
    CABLE_VOL_DB_MIN,
    Math.min(CABLE_VOL_DB_MAX, 20 * Math.log10(v))
  );
}

export function cableDbToPct(db: number): number {
  return (
    ((db - CABLE_VOL_DB_MIN) / (CABLE_VOL_DB_MAX - CABLE_VOL_DB_MIN)) * 100
  );
}

export type CableVolZone = "low" | "ok" | "high" | "idle";

export function cableZoneForDb(db: number): CableVolZone {
  if (db <= CABLE_VOL_DB_MIN + 0.5) return "idle";
  if (db < CABLE_VOL_DB_LOW) return "low";
  if (db > CABLE_VOL_DB_HIGH) return "high";
  return "ok";
}

export function cableZoneForLevel(level: number): CableVolZone {
  return cableZoneForDb(cableLevelToDb(level));
}
