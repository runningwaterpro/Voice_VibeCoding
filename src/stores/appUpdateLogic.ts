import type { AppUpdateInfo } from "../types";

/** 启动自动弹窗推迟，避免抢在桥接/语音初始化之前挡操作 */
export const APP_UPDATE_AUTO_OPEN_DELAY_MS = 12_000;

/** 后端总开关关闭时（source=disabled），不展示任何更新入口结果 */
export function isUpdateCheckDisabled(
  info: AppUpdateInfo | null | undefined,
): boolean {
  return info?.source === "disabled";
}

/** 顶栏角标、启动自动弹窗：有新版本且未忽略自动提醒 */
export function shouldShowPassivePrompt(info: AppUpdateInfo | null | undefined): boolean {
  if (isUpdateCheckDisabled(info)) return false;
  return Boolean(info?.updateAvailable && !info?.promptSuppressed);
}

/** 被动检测是否应自动打开弹窗（另需 session 未 dismiss） */
export function shouldAutoOpenModal(info: AppUpdateInfo | null | undefined): boolean {
  return shouldShowPassivePrompt(info);
}

/**
 * 本会话是否允许自动弹窗。
 * 「关闭」或「不再提醒」都会写入 session dismiss，防止启动检测晚到事件再次弹开。
 */
export function shouldAutoOpenForSession(
  latestVersion: string | null | undefined,
  dismissedVersion: string | null | undefined,
): boolean {
  if (!latestVersion) return false;
  return dismissedVersion !== latestVersion;
}

/** 设置页主动检查：只要有新版本就应弹窗（含已忽略）；总开关关闭时不弹 */
export function shouldOpenModalFromManualCheck(info: AppUpdateInfo | null | undefined): boolean {
  if (isUpdateCheckDisabled(info)) return false;
  return Boolean(info?.updateAvailable);
}
