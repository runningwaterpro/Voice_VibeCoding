//! 界面可见性 —— 唯一定义处。
//!
//! Windows 把窗口最小化到托盘后，WebView2 不保证派发 `visibilitychange`。实测
//! 最小化后前端的 2 秒轮询仍在跑（后端每秒数百次注册表读），说明不能靠
//! `document.hidden` 判断"界面还需要刷新"。
//!
//! 所以可见性由窗口事件在 Rust 侧判定，前端只跟随：
//! - [`set_window_visible`] 由窗口事件维护（最小化/还原/重建）。
//! - [`set_ui_hidden`] 接受前端自报的隐藏状态，只能收紧不能放宽。
//!
//! 需要绘制界面或刷新界面状态的地方，统一问 [`ui_visible`]。

use std::sync::atomic::{AtomicBool, Ordering};

/// 窗口是否可见且未最小化。由窗口事件维护。
static WINDOW_VISIBLE: AtomicBool = AtomicBool::new(true);

/// 前端自报的隐藏（`document.hidden`）。只能收紧可见性。
static UI_HIDDEN: AtomicBool = AtomicBool::new(false);

pub fn set_window_visible(visible: bool) {
    WINDOW_VISIBLE.store(visible, Ordering::SeqCst);
}

pub fn set_ui_hidden(hidden: bool) {
    UI_HIDDEN.store(hidden, Ordering::SeqCst);
}

/// 界面是否处于"用户看得见"的状态。
pub fn ui_visible() -> bool {
    WINDOW_VISIBLE.load(Ordering::SeqCst) && !UI_HIDDEN.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_when_window_minimized_or_page_hidden() {
        set_window_visible(true);
        set_ui_hidden(false);
        assert!(ui_visible());

        set_window_visible(false);
        assert!(!ui_visible(), "最小化后必须为隐藏");

        set_window_visible(true);
        set_ui_hidden(true);
        assert!(!ui_visible(), "页面自报隐藏后必须为隐藏");

        set_ui_hidden(false);
        assert!(ui_visible());
    }
}
