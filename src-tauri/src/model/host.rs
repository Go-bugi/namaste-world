use crate::agent::domain::model::Environment;
use std::future::Future;

/// 모델 목록을 묻는 CLI 실행. `platform/`이 OS마다 구현하고, 테스트는 가짜 구현을 쓴다.
pub trait ModelHost: Sync {
    /// Claude를 stream-json 모드로 띄워 초기화 요청 한 줄(`request`)을 쓰고, `accept`가 Some을 돌려주는 줄이 나올 때까지 읽는다.
    /// 훅과 MCP를 끄고 세션을 저장하지 않는다. 프로세스는 응답을 받거나 future가 버려지면 죽인다.
    fn claude_init<T: Send>(
        &self,
        environment: &Environment,
        path: &str,
        request: &str,
        accept: impl Fn(&str) -> Option<T> + Send + Sync,
    ) -> impl Future<Output = Option<T>> + Send;

    /// `codex debug models`의 stdout. 실패하면 None.
    fn codex_models(
        &self,
        environment: &Environment,
        path: &str,
    ) -> impl Future<Output = Option<String>> + Send;

    /// 그 환경의 `~/.codex/config.toml` 내용. 없거나 읽지 못하면 None.
    fn codex_config(
        &self,
        environment: &Environment,
    ) -> impl Future<Output = Option<String>> + Send;
}

/// Claude를 프롬프트 없이 초기화 요청만 받는 stream-json 모드로 띄우는 인자. 뒤에 `--settings`의 값을 붙인다.
/// MCP 서버를 띄우지 않고(`--strict-mcp-config`) 세션 기록을 남기지 않는다.
pub const CLAUDE_INIT_ARGS: [&str; 9] = [
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
    "--strict-mcp-config",
    "--no-session-persistence",
    "--settings",
];

/// `--settings`로 넘기는 값. 조회하는 동안 사용자 훅(앱 훅 포함)이 실행되지 않게 한다.
pub const CLAUDE_INIT_SETTINGS: &str = r#"{"disableAllHooks":true}"#;

/// Codex 모델 목록 명령의 인자.
pub const CODEX_MODELS_ARGS: [&str; 2] = ["debug", "models"];
