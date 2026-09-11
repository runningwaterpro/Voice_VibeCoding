import { describe, expect, it } from "vitest";
import {
  CABLE_VOL_DB_HIGH,
  CABLE_VOL_DB_LOW,
  cableDbToPct,
  cableLevelToDb,
  cableZoneForDb,
  cableZoneForLevel,
} from "./cableVolMeter";

describe("cableVolMeter", () => {
  it("maps silence to floor dB", () => {
    expect(cableLevelToDb(0)).toBe(-42);
  });

  it("maps full scale to 0 dBFS", () => {
    expect(cableLevelToDb(1)).toBe(0);
  });

  it("classifies zones on dB thresholds", () => {
    expect(cableZoneForDb(-42)).toBe("idle");
    expect(cableZoneForDb(-36)).toBe("low");
    expect(cableZoneForDb(-12)).toBe("ok");
    expect(cableZoneForDb(-3)).toBe("high");
    expect(CABLE_VOL_DB_LOW).toBe(-28);
    expect(CABLE_VOL_DB_HIGH).toBe(-6);
  });

  it("converts level to pct monotonically", () => {
    expect(cableDbToPct(-42)).toBe(0);
    expect(cableDbToPct(0)).toBe(100);
    // 0.5 linear ≈ -6.02 dBFS → ok (not yet high)
    expect(cableZoneForLevel(0.5)).toBe("ok");
  });
});
