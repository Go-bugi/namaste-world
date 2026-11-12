use crate::agent::detect::{detect, DETECTION_LIMIT};
use crate::agent::domain::model::{Detection, Trigger};
use crate::platform::AgentHost;
use crate::state::AgentDetection;
use tauri::Manager;

/// 이 OS의 환경을 감지한다. 상한 시간 안에 끝난다.
pub async fn run(trigger: Trigger) -> Detection {
    let host = AgentHost::new(trigger).await;
    detect(&host, trigger, DETECTION_LIMIT).await
}

/// 앱 시작 때 감지를 한 번 돌려 결과를 상태에 넣는다. 멈춘 WSL 배포판은 켜지 않는다.
pub fn spawn_startup_detection(app: tauri::AppHandle) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let detection = run(Trigger::Startup).await;
        app.state::<AgentDetection>().publish(detection);
    })
}
