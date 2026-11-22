//! 창별 Rust 코드. 창을 고르고 띄우는 일만 하고, 위치 계산은 `placement`에 맡긴다.

pub mod main;

use crate::placement::window_to_show;
use tauri::Manager;

/// 시작 설정 창의 label.
pub const ONBOARDING: &str = "onboarding";

/// 앱을 다시 불러낼 때(macOS 독 클릭, 다시 실행) 지금 쓰는 창을 보이고 앞으로 가져온다.
/// 작은 창으로 넘어간 뒤면 작은 창을, 그 전이면 시작 설정 창을 보인다.
/// 작은 창은 화면을 벗어나 있으면 보이는 곳으로 옮긴 뒤 보인다(FR-WINDOW-006).
pub fn show_current(app: &tauri::AppHandle) {
    let label = window_to_show(app.get_webview_window(main::LABEL).is_some());
    if let Some(window) = app.get_webview_window(label) {
        // 최소화를 먼저 풀어야 작은 창의 실제 위치를 읽을 수 있다
        let _ = window.unminimize();
        if label == main::LABEL {
            let _ = main::keep_visible(app);
        }
        let _ = window.show();
        let _ = window.set_focus();
    }
}
