//! 창별 Rust 코드. 창을 고르고 띄우는 일만 하고, 위치 계산은 `placement`에 맡긴다.

pub mod main;

use crate::placement::window_to_show;
use tauri::Manager;

/// 시작 설정 창의 label.
pub const ONBOARDING: &str = "onboarding";

/// 앱을 다시 불러낼 때(macOS 독 클릭, 다시 실행) 지금 쓰는 창을 보이고 앞으로 가져온다.
/// 작은 창으로 넘어간 뒤면 작은 창을, 그 전이면 시작 설정 창을 보인다.
pub fn show_current(app: &tauri::AppHandle) {
    let label = window_to_show(app.get_webview_window(main::LABEL).is_some());
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
