use crate::agent::domain::model::Environment;
use std::future::Future;

/// 끝난 CLI의 출력. 종료 코드는 로그인 판단에 쓰지 않아 담지 않는다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    pub stdout: String,
    pub stderr: String,
}

/// 로그인 상태를 묻는 CLI 실행. `platform/`이 OS마다 구현하고, 테스트는 가짜 구현을 쓴다.
pub trait LoginHost: Sync {
    /// `environment`에서 `path`를 `args`로 실행하고 끝나면 출력을 준다. 종료 코드가 0이 아니어도 출력을 준다.
    /// 띄우지 못하면 None. future가 버려지면 프로세스 트리를 죽인다.
    fn run(
        &self,
        environment: &Environment,
        path: &str,
        args: &[&str],
    ) -> impl Future<Output = Option<CliOutput>> + Send;
}

/// Claude Code의 로그인 상태 명령 인자.
pub const CLAUDE_STATUS_ARGS: [&str; 3] = ["auth", "status", "--json"];

/// Codex의 로그인 상태 명령 인자.
pub const CODEX_STATUS_ARGS: [&str; 2] = ["login", "status"];
