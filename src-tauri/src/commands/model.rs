use crate::agent::domain::model::{AgentKind, AgentStatus, Environment};
use crate::model::domain::catalog::model_list;
use crate::model::domain::model::ModelList;
use crate::model::query::{query_models, MODEL_QUERY_LIMIT};
use crate::platform::ModelCli;
use crate::state::AgentDetection;

/// 한 에이전트와 실행 환경의 모델 목록. 실패하지 않으며, 조회하지 못하면 고정 목록을 준다.
/// 실행 파일 경로는 보관한 감지 결과에서 찾는다. 설치됨이 아니면 조회하지 않고 고정 목록을 준다.
#[tauri::command]
pub async fn agent_models(
    agent: AgentKind,
    environment: Environment,
    detection: tauri::State<'_, AgentDetection>,
) -> Result<ModelList, ()> {
    let path = detection.latest().await.and_then(|detection| {
        detection
            .entries
            .into_iter()
            .find_map(|entry| match entry.status {
                AgentStatus::Installed { path, .. }
                    if entry.agent == agent && entry.environment == environment =>
                {
                    Some(path)
                }
                _ => None,
            })
    });
    let Some(path) = path else {
        return Ok(model_list(agent, None, None));
    };
    let host = ModelCli::new().await;
    Ok(query_models(&host, agent, &environment, &path, MODEL_QUERY_LIMIT).await)
}
