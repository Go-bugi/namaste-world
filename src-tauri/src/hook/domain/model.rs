use crate::agent::domain::model::{AgentKind, Environment};
use serde::Serialize;
use ts_rs::TS;

/// 설치하지 못한 이유.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum InstallFailure {
    /// 설정 파일을 JSON이나 TOML로 해석할 수 없거나 모양이 예상과 다르다.
    InvalidConfig { file: String },
    /// 설정 파일에 쓸 권한이 없다.
    NotWritable { file: String },
    /// 사용자가 Codex 설정에서 훅 기능을 꺼 두었다.
    CodexHooksDisabled { file: String },
    /// 그 밖의 파일이나 프로세스 실패.
    Io { message: String },
}

/// 한 에이전트와 실행 환경의 설치 결과.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum InstallStatus {
    /// 이번 실행에서 파일을 하나 이상 바꿨다.
    Installed,
    /// 바꿀 파일이 없었다.
    AlreadyInstalled,
    Failed {
        reason: InstallFailure,
    },
}

/// 설치 결과의 한 줄. 에이전트와 실행 환경의 조합마다 하나다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct InstallEntry {
    pub agent: AgentKind,
    pub environment: Environment,
    pub status: InstallStatus,
}

/// 설치 한 번의 결과.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HookInstallation {
    pub entries: Vec<InstallEntry>,
}

/// renderer에 넘기는 훅 이벤트.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HookEvent {
    pub agent: AgentKind,
    /// 본문의 `hook_event_name`. 없으면 URL의 이벤트 이름.
    pub event: String,
    /// 받은 시각(Unix ms).
    pub received_at: u64,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// 1MB 이하인 본문의 원래 입력 전체.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    /// 본문이 1MB를 넘어 `payload`를 뺐다.
    pub truncated: bool,
}

/// 훅을 설치할 곳. 경로는 모두 그 환경 안의 표기다(macOS·WSL은 POSIX, Windows는 `C:\...`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallTarget {
    pub agent: AgentKind,
    pub environment: Environment,
    /// 감지 결과의 버전.
    pub version: String,
    pub home: String,
    /// 그 환경에서 훅 스크립트가 읽을 엔드포인트 파일 경로.
    pub endpoint_path: String,
}

/// 훅 명령을 실행하는 셸 계열. macOS와 WSL은 Posix, Windows는 Windows다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellFamily {
    Posix,
    Windows,
}

impl ShellFamily {
    pub fn of(environment: &Environment) -> ShellFamily {
        match environment {
            Environment::Windows => ShellFamily::Windows,
            Environment::Macos | Environment::Wsl { .. } => ShellFamily::Posix,
        }
    }
}

/// 설정 파일 병합 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merge {
    /// 지금 내용 그대로 두면 된다.
    Unchanged,
    /// 이 내용으로 바꿔야 한다.
    Updated(String),
}

/// 설정 파일을 병합하지 못한 이유.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeError {
    InvalidJson,
    InvalidToml,
    /// 최상위나 `hooks`, 이벤트 배열이 예상한 모양이 아니다.
    NotObject,
    /// Codex `[features] hooks`(또는 `codex_hooks`)가 false다.
    HooksDisabled,
}
