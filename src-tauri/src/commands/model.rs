use crate::agent::domain::assemble::installed_path;
use crate::agent::domain::model::{AgentKind, Environment};
use crate::model::domain::catalog::model_list;
use crate::model::domain::model::ModelList;
use crate::model::query::{query_models, MODEL_QUERY_LIMIT};
use crate::platform::{self, AgentCli};
use crate::state::{AgentDetection, ShellEnvState};

/// 한 에이전트와 실행 환경의 모델 목록. 실패하지 않으며, 조회하지 못하면 고정 목록을 준다.
/// 실행 파일 경로는 보관한 감지 결과에서 찾는다. 설치됨이 아니면 조회하지 않고 고정 목록을 준다.
#[tauri::command]
pub async fn agent_models(
    agent: AgentKind,
    environment: Environment,
    detection: tauri::State<'_, AgentDetection>,
    shell_env: tauri::State<'_, ShellEnvState>,
) -> Result<ModelList, ()> {
    let path = detection
        .latest()
        .await
        .and_then(|detection| installed_path(&detection, agent, &environment));
    let Some(path) = path else {
        return Ok(model_list(agent, None, None));
    };
    // 감지 때 읽은 환경을 쓴다. 셸을 다시 띄우지 않는다(FR-AGENT-015)
    let host = AgentCli::new(shell_env.env_or(platform::app_env));
    Ok(query_models(&host, agent, &environment, &path, MODEL_QUERY_LIMIT).await)
}
