use serde::Serialize;
use ts_rs::TS;

/// 한 에이전트와 실행 환경의 로그인 상태.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum LoginStatus {
    /// CLI가 로그인되어 있다고 알렸다. 방식은 CLI가 알려 줄 때만 담는다.
    LoggedIn {
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        method: Option<LoginMethod>,
    },
    /// CLI가 로그인되어 있지 않다고 알렸다.
    LoggedOut,
    /// 로그인 여부를 알아내지 못했다.
    Failed { reason: LoginFailure },
}

/// 로그인 방식.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LoginMethod {
    /// claude.ai, ChatGPT 구독
    Subscription,
    ApiKey,
    /// Bedrock, Vertex 같은 클라우드 제공자
    Cloud,
    /// 앱이 모르는 방식
    Other,
}

/// 로그인 여부를 알아내지 못한 이유.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum LoginFailure {
    /// 보관한 감지 결과에 그 항목이 설치됨으로 없다.
    NotInstalled,
    /// CLI를 띄우지 못했다.
    LaunchFailed,
    /// 시간 상한(SC-AGENT-002) 안에 끝나지 않았다.
    Timeout,
    /// 출력으로 로그인 여부를 판단할 수 없다.
    UnreadableOutput,
}
