// 감지 로직은 OS마다 쓰는 부분이 달라(예: WSL 해석은 Windows만) 공개 모듈로 두어 다른 OS 빌드에서 미사용 경고가 나지 않게 한다
pub mod agent;
mod commands;
mod events;
pub mod hook;
mod platform;
mod state;
mod tasks;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 앱을 하나만 실행한다. 다시 실행하면 새 앱을 띄우지 않고 켜져 있는 앱의 main 창을 앞으로 가져온다.
        // 다른 플러그인보다 먼저 등록해야 두 번째 프로세스가 다른 초기화를 하기 전에 끝난다.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        // 외부 링크(target="_blank")는 opener 플러그인이 기본 브라우저로 연다.
        .plugin(tauri_plugin_opener::init())
        .manage(state::AgentDetection::new())
        .manage(state::HookInstallState::new())
        .setup(|app| {
            tasks::agent_detection::spawn_startup_detection(app.handle().clone());
            tasks::hook_server::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::ping,
            commands::agent::agent_detection,
            commands::agent::redetect_agents,
            commands::hook::hook_installation
        ])
        .on_window_event(platform::on_window_event)
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                tasks::hook_server::remove_endpoint();
            }
            platform::on_run_event(app, event);
        });
}
