use serde::Serialize;
use std::collections::BTreeMap;
use ts_rs::TS;

/// 감지하는 코딩 에이전트.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AgentKind {
    Claude,
    Codex,
}

impl AgentKind {
    /// 결과를 낼 때 쓰는 순서. 항목은 항상 이 순서로 나온다.
    pub const ALL: [AgentKind; 2] = [AgentKind::Claude, AgentKind::Codex];

    /// 실행 파일 이름(확장자 없음).
    pub fn command(self) -> &'static str {
        match self {
            AgentKind::Claude => "claude",
            AgentKind::Codex => "codex",
        }
    }
}

/// 에이전트가 실행되는 환경. WSL은 배포판마다 다른 환경이다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum Environment {
    Macos,
    Windows,
    Wsl { distro: String },
}

/// 확인하지 못한 이유.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum FailureReason {
    /// 감지 상한 시간 안에 확인하지 못했다.
    Timeout,
    /// 실행 파일은 있지만 `--version`이 실패했거나 버전을 읽을 수 없었다.
    VersionUnreadable,
    /// 환경 자체를 확인하지 못했다(`wsl.exe` 실패 등).
    EnvironmentError { message: String },
}

/// 한 환경에서 한 에이전트의 상태.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum AgentStatus {
    Installed {
        path: String,
        version: String,
    },
    NotInstalled,
    Failed {
        reason: FailureReason,
    },
    /// 멈춘 WSL 배포판이라 앱 시작 감지에서 확인하지 않았다.
    Stopped,
}

/// 감지 결과의 한 줄. 에이전트와 환경의 조합마다 하나다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AgentEntry {
    pub agent: AgentKind,
    pub environment: Environment,
    pub status: AgentStatus,
}

/// 감지 한 번의 결과.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Detection {
    pub entries: Vec<AgentEntry>,
}

/// 감지를 시작한 쪽. 사용자가 요청한 감지만 멈춘 WSL 배포판을 켤 수 있다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Startup,
    UserRequested,
}

/// 확인할 환경과 그 환경이 지금 실행 중인지. macOS와 Windows는 항상 실행 중이다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub environment: Environment,
    pub running: bool,
}

/// 실행 파일 하나를 찾은 결과. `version_output`이 `None`이면 `--version`이 실패했거나 시간 안에 끝나지 않았다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub path: String,
    pub version_output: Option<String>,
}

/// 한 환경의 프로브 결과. 찾지 못한 에이전트는 맵에 없다. `Err`면 환경 자체를 확인하지 못했다.
pub type EnvProbe = Result<BTreeMap<AgentKind, Found>, FailureReason>;

/// 실행 파일을 찾는 호스트 OS. 디렉터리 목록과 파일 이름 규칙이 다르다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOs {
    Macos,
    Windows,
}
