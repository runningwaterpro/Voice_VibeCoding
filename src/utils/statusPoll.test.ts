import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cancelPoll,
  pollEvery,
  pollsVisible,
  resetPollsForTest,
  setPollVisible,
} from "./statusPoll";

describe("statusPoll", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    resetPollsForTest();
  });

  afterEach(() => {
    resetPollsForTest();
    vi.useRealTimers();
  });

  it("按各自间隔执行任务", () => {
    const fast = vi.fn();
    const slow = vi.fn();
    pollEvery(fast, 1000);
    pollEvery(slow, 2000);

    vi.advanceTimersByTime(1000);
    expect(fast).toHaveBeenCalledTimes(1);
    expect(slow).toHaveBeenCalledTimes(0);

    vi.advanceTimersByTime(1000);
    expect(fast).toHaveBeenCalledTimes(2);
    expect(slow).toHaveBeenCalledTimes(1);
  });

  it("隐藏后不再执行，恢复时立刻补跑一轮", () => {
    const fn = vi.fn();
    pollEvery(fn, 1000);

    setPollVisible(false);
    expect(pollsVisible()).toBe(false);
    vi.advanceTimersByTime(10_000);
    expect(fn).toHaveBeenCalledTimes(0);

    setPollVisible(true);
    expect(fn).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(1000);
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("任务抛错不影响其它任务", () => {
    const bad = vi.fn(() => {
      throw new Error("boom");
    });
    const good = vi.fn();
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    pollEvery(bad, 1000);
    pollEvery(good, 1000);

    vi.advanceTimersByTime(1000);
    expect(bad).toHaveBeenCalledTimes(1);
    expect(good).toHaveBeenCalledTimes(1);
    warn.mockRestore();
  });

  it("取消后不再执行", () => {
    const fn = vi.fn();
    const task = pollEvery(fn, 1000);
    vi.advanceTimersByTime(1000);
    expect(fn).toHaveBeenCalledTimes(1);

    cancelPoll(task);
    vi.advanceTimersByTime(5000);
    expect(fn).toHaveBeenCalledTimes(1);
  });
});
