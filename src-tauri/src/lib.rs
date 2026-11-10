mod commands;
mod platform;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 외부 링크(target="_blank")는 opener 플러그인이 기본 브라우저로 연다.
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::app::ping])
        .on_window_event(platform::on_window_event)
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(platform::on_run_event);
}
