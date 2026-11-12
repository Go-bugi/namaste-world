use crate::agent::domain::model::{Detection, Trigger};
use crate::state::AgentDetection;
use crate::tasks::agent_detection;

/// 보관한 감지 결과를 준다. 앱 시작 감지가 아직이면 끝날 때까지 기다린다.
/// `State`를 받는 async 명령은 `Result`를 돌려줘야 해서 에러 타입을 `()`로 둔다. 실제로는 실패하지 않는다.
#[tauri::command]
pub async fn agent_detection(state: tauri::State<'_, AgentDetection>) -> Result<Detection, ()> {
    state.latest().await.ok_or(())
}

/// 새로 감지해 보관한 결과를 바꾸고 그 결과를 준다. 사용자 요청이므로 멈춘 WSL 배포판도 켜서 확인한다.
#[tauri::command]
pub async fn redetect_agents(state: tauri::State<'_, AgentDetection>) -> Result<Detection, ()> {
    let detection = agent_detection::run(Trigger::UserRequested).await;
    state.publish(detection.clone());
    Ok(detection)
}
