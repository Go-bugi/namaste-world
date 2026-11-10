use tauri::Manager;

/// 창을 닫아도 앱은 종료하지 않고 창만 숨긴다. 종료는 Cmd+Q로 한다.
pub fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
}

/// 독 아이콘을 클릭했을 때 보이는 창이 없으면 숨긴 창을 다시 띄운다.
pub fn on_run_event(app: &tauri::AppHandle, event: tauri::RunEvent) {
    if let tauri::RunEvent::Reopen {
        has_visible_windows: false,
        ..
    } = event
    {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}
