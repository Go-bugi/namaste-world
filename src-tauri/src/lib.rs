// 감지 로직은 OS마다 쓰는 부분이 달라(예: WSL 해석은 Windows만) 공개 모듈로 두어 다른 OS 빌드에서 미사용 경고가 나지 않게 한다
pub mod agent;
mod commands;
mod platform;
mod state;
mod tasks;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 외부 링크(target="_blank")는 opener 플러그인이 기본 브라우저로 연다.
        .plugin(tauri_plugin_opener::init())
        .manage(state::AgentDetection::new())
        .setup(|app| {
            tasks::agent_detection::spawn_startup_detection(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::ping,
            commands::agent::agent_detection,
            commands::agent::redetect_agents
        ])
        .on_window_event(platform::on_window_event)
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(platform::on_run_event);
}
