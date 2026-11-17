use crate::agent::domain::assemble::installed_path;
use crate::agent::domain::model::{AgentKind, Environment};
use crate::login::check::{check_login, LOGIN_CHECK_LIMIT};
use crate::login::domain::model::{LoginFailure, LoginStatus};
use crate::platform::AgentCli;
use crate::state::AgentDetection;

/// 한 에이전트와 실행 환경의 로그인 상태. 실패하지 않으며, 알아내지 못하면 `Failed`에 이유를 담는다.
/// 실행 파일 경로는 보관한 감지 결과에서 찾고, 설치됨이 아니면 CLI를 실행하지 않는다.
#[tauri::command]
pub async fn agent_login_status(
    agent: AgentKind,
    environment: Environment,
    detection: tauri::State<'_, AgentDetection>,
) -> Result<LoginStatus, ()> {
    let path = detection
        .latest()
        .await
        .and_then(|detection| installed_path(&detection, agent, &environment));
    let Some(path) = path else {
        return Ok(LoginStatus::Failed {
            reason: LoginFailure::NotInstalled,
        });
    };
    let host = AgentCli::new().await;
    Ok(check_login(&host, agent, &environment, &path, LOGIN_CHECK_LIMIT).await)
}
