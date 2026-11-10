#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{on_run_event, on_window_event};

/// macOS가 아니면 창 이벤트는 기본 동작을 따른다.
#[cfg(not(target_os = "macos"))]
pub fn on_window_event(_window: &tauri::Window, _event: &tauri::WindowEvent) {}

/// macOS가 아니면 앱 이벤트에서 따로 할 일이 없다.
#[cfg(not(target_os = "macos"))]
pub fn on_run_event(_app: &tauri::AppHandle, _event: tauri::RunEvent) {}
