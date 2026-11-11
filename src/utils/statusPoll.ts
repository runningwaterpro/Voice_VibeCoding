/**
 * 统一轮询调度：一个定时器跑多个任务，各自保留自己的间隔。
 *
 * 为什么不各写各的 setInterval：
 * - 每个状态查询都会让后端重探一遍环境，两处独立轮询等于把同一份开销跑两遍；
 * - 界面隐藏（托盘/最小化）时只有一个开关能停掉全部轮询。实测 WebView2 在
 *   托盘态不一定派发 `visibilitychange`，所以隐藏判定来自 Rust 窗口事件
 *   （见 `ui-visibility` 事件 / `get_ui_visible` 命令）。
 *
 * 计时用 tick 累加，不读壁钟：定时器被节流或系统休眠都不会让任务提前堆积。
 */

export interface PollTask {
  readonly fn: () => void;
  readonly ms: number;
  /** 已累计的时间（毫秒），到 `ms` 触发一次并清零。 */
  elapsed: number;
}

const TICK_MS = 500;
const tasks = new Set<PollTask>();
let timer: ReturnType<typeof setInterval> | null = null;
let visible = true;

function tick() {
  if (!visible) return;
  for (const task of tasks) {
    task.elapsed += TICK_MS;
    if (task.elapsed < task.ms) continue;
    task.elapsed = 0;
    try {
      task.fn();
    } catch (e) {
      console.warn("poll task failed", e);
    }
  }
}

function ensureTimer() {
  if (timer || !visible || tasks.size === 0) return;
  timer = setInterval(tick, TICK_MS);
}

function stopTimer() {
  if (timer) clearInterval(timer);
  timer = null;
}

/**
 * 注册周期性任务。首次执行在 `ms` 之后；需要立刻刷新的调用方自己先跑一次。
 */
export function pollEvery(fn: () => void, ms: number): PollTask {
  const task: PollTask = { fn, ms: Math.max(TICK_MS, ms), elapsed: 0 };
  tasks.add(task);
  ensureTimer();
  return task;
}

export function cancelPoll(task: PollTask | null | undefined) {
  if (!task) return;
  tasks.delete(task);
  if (tasks.size === 0) stopTimer();
}

/**
 * 界面可见性开关。隐藏时暂停全部任务，恢复时立刻补跑一轮。
 */
export function setPollVisible(next: boolean) {
  if (visible === next) return;
  visible = next;
  if (!next) {
    stopTimer();
    return;
  }
  for (const task of tasks) task.elapsed = task.ms;
  ensureTimer();
  tick();
}

export function pollsVisible(): boolean {
  return visible;
}

/** 仅测试用：清空调度器状态。 */
export function resetPollsForTest() {
  stopTimer();
  tasks.clear();
  visible = true;
}
