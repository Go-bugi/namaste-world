use super::domain::model::{LoginFailure, LoginStatus};
use super::domain::parse::{claude_status, codex_status};
use super::host::{LoginHost, CLAUDE_STATUS_ARGS, CODEX_STATUS_ARGS};
use crate::agent::domain::model::{AgentKind, Environment};
use std::time::Duration;

/// 로그인 상태 확인 한 번의 상한. SC-AGENT-002
pub const LOGIN_CHECK_LIMIT: Duration = Duration::from_secs(5);

/// 한 항목의 로그인 상태. `limit` 안에 끝나지 않으면 시간 초과, 띄우지 못하면 실행 실패,
/// 출력을 해석하지 못하면 해석 실패로 낸다.
pub async fn check_login(
    host: &impl LoginHost,
    agent: AgentKind,
    environment: &Environment,
    path: &str,
    limit: Duration,
) -> LoginStatus {
    let args: &[&str] = match agent {
        AgentKind::Claude => &CLAUDE_STATUS_ARGS,
        AgentKind::Codex => &CODEX_STATUS_ARGS,
    };
    let failed = |reason| LoginStatus::Failed { reason };
    let output = match tokio::time::timeout(limit, host.run(environment, path, args)).await {
        Err(_) => return failed(LoginFailure::Timeout),
        Ok(None) => return failed(LoginFailure::LaunchFailed),
        Ok(Some(output)) => output,
    };
    let status = match agent {
        AgentKind::Claude => claude_status(&output.stdout),
        AgentKind::Codex => codex_status(&output.stdout, &output.stderr),
    };
    status.unwrap_or_else(|| failed(LoginFailure::UnreadableOutput))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::domain::model::{LoginFailure, LoginMethod};
    use crate::login::host::{CliOutput, CLAUDE_STATUS_ARGS, CODEX_STATUS_ARGS};
    use tokio::time::Instant;

    /// 가짜 CLI의 반응.
    #[derive(Clone)]
    enum Reply {
        /// `after`만큼 걸려 끝나고 출력을 준다.
        Output { after: Duration, output: CliOutput },
        /// 띄우지 못한다.
        LaunchFailed,
        /// 끝나지 않는다.
        Hang,
    }

    /// 정해 둔 환경, 경로, 인자로 불렸을 때만 `reply`대로 반응하고, 다르게 불리면 띄우지 못한 것으로 답한다.
    struct FakeHost {
        environment: Environment,
        path: &'static str,
        args: &'static [&'static str],
        reply: Reply,
    }

    impl LoginHost for FakeHost {
        async fn run(
            &self,
            environment: &Environment,
            path: &str,
            args: &[&str],
        ) -> Option<CliOutput> {
            if environment != &self.environment || path != self.path || args != self.args {
                return None;
            }
            match self.reply.clone() {
                Reply::Output { after, output } => {
                    tokio::time::sleep(after).await;
                    Some(output)
                }
                Reply::LaunchFailed => None,
                Reply::Hang => std::future::pending().await,
            }
        }
    }

    fn ubuntu() -> Environment {
        Environment::Wsl {
            distro: "Ubuntu".to_string(),
        }
    }

    fn output(stdout: &str, stderr: &str) -> CliOutput {
        CliOutput {
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        }
    }

    fn claude_host(environment: Environment, reply: Reply) -> FakeHost {
        FakeHost {
            environment,
            path: "/usr/local/bin/claude",
            args: &CLAUDE_STATUS_ARGS,
            reply,
        }
    }

    fn codex_host(environment: Environment, reply: Reply) -> FakeHost {
        FakeHost {
            environment,
            path: "/usr/bin/codex",
            args: &CODEX_STATUS_ARGS,
            reply,
        }
    }

    fn immediately(stdout: &str, stderr: &str) -> Reply {
        Reply::Output {
            after: Duration::ZERO,
            output: output(stdout, stderr),
        }
    }

    fn failed(reason: LoginFailure) -> LoginStatus {
        LoginStatus::Failed { reason }
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_009_claude_subscription_is_logged_in() {
        let host = claude_host(
            Environment::Macos,
            immediately(
                r#"{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty"}"#,
                "",
            ),
        );
        let got = check_login(
            &host,
            AgentKind::Claude,
            &Environment::Macos,
            "/usr/local/bin/claude",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(
            got,
            LoginStatus::LoggedIn {
                method: Some(LoginMethod::Subscription)
            }
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_009_wsl_codex_logged_out() {
        // 로그인 안 됨에서 Codex는 결과 문장을 stderr로 낸다
        let host = codex_host(ubuntu(), immediately("", "Not logged in\n"));
        let got = check_login(
            &host,
            AgentKind::Codex,
            &ubuntu(),
            "/usr/bin/codex",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(got, LoginStatus::LoggedOut);
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_009_windows_codex_api_key() {
        let host = codex_host(
            Environment::Windows,
            immediately("", "Logged in using an API key - sk-proj-***ABCD\n"),
        );
        let got = check_login(
            &host,
            AgentKind::Codex,
            &Environment::Windows,
            "/usr/bin/codex",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(
            got,
            LoginStatus::LoggedIn {
                method: Some(LoginMethod::ApiKey)
            }
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_010_launch_failure_is_failed_not_logged_out() {
        let host = codex_host(Environment::Windows, Reply::LaunchFailed);
        let got = check_login(
            &host,
            AgentKind::Codex,
            &Environment::Windows,
            "/usr/bin/codex",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(got, failed(LoginFailure::LaunchFailed));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_010_unreadable_output_is_failed_not_logged_out() {
        let claude = claude_host(
            Environment::Macos,
            immediately("Usage: claude [options]", ""),
        );
        let got = check_login(
            &claude,
            AgentKind::Claude,
            &Environment::Macos,
            "/usr/local/bin/claude",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(got, failed(LoginFailure::UnreadableOutput));

        let codex = codex_host(
            Environment::Macos,
            immediately("", "something went wrong\n"),
        );
        let got = check_login(
            &codex,
            AgentKind::Codex,
            &Environment::Macos,
            "/usr/bin/codex",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(got, failed(LoginFailure::UnreadableOutput));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_010_hanging_cli_is_timeout() {
        let host = codex_host(ubuntu(), Reply::Hang);
        let got = check_login(
            &host,
            AgentKind::Codex,
            &ubuntu(),
            "/usr/bin/codex",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(got, failed(LoginFailure::Timeout));
    }

    #[tokio::test(start_paused = true)]
    async fn sc_agent_002_hanging_cli_fails_within_five_seconds() {
        let host = claude_host(Environment::Macos, Reply::Hang);
        let started = Instant::now();
        let got = check_login(
            &host,
            AgentKind::Claude,
            &Environment::Macos,
            "/usr/local/bin/claude",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(got, failed(LoginFailure::Timeout));
        assert!(started.elapsed() <= Duration::from_secs(5));
    }

    #[tokio::test(start_paused = true)]
    async fn sc_agent_002_answer_within_limit_is_used() {
        // 상한보다 일찍 끝난 CLI의 결과는 시간 초과가 아니라 그 결과로 낸다
        let host = codex_host(
            Environment::Macos,
            Reply::Output {
                after: Duration::from_secs(4),
                output: output("", "Logged in using ChatGPT\n"),
            },
        );
        let got = check_login(
            &host,
            AgentKind::Codex,
            &Environment::Macos,
            "/usr/bin/codex",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(
            got,
            LoginStatus::LoggedIn {
                method: Some(LoginMethod::Subscription)
            }
        );
    }

    #[tokio::test(start_paused = true)]
    async fn sc_agent_002_answer_after_limit_is_timeout() {
        let host = codex_host(
            Environment::Macos,
            Reply::Output {
                after: Duration::from_secs(6),
                output: output("", "Logged in using ChatGPT\n"),
            },
        );
        let got = check_login(
            &host,
            AgentKind::Codex,
            &Environment::Macos,
            "/usr/bin/codex",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(got, failed(LoginFailure::Timeout));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_009_runs_cli_with_detected_path_in_its_environment() {
        // 가짜 CLI는 감지 경로, 요청한 환경, 상태 명령 인자로 불릴 때만 답한다.
        // 다른 경로나 환경으로 부르면 띄우지 못한 것이 되어 로그인됨이 나오지 않는다
        let host = codex_host(ubuntu(), immediately("", "Logged in using ChatGPT\n"));
        let right = check_login(
            &host,
            AgentKind::Codex,
            &ubuntu(),
            "/usr/bin/codex",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(
            right,
            LoginStatus::LoggedIn {
                method: Some(LoginMethod::Subscription)
            }
        );

        let other_path = check_login(
            &host,
            AgentKind::Codex,
            &ubuntu(),
            "/opt/codex",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(other_path, failed(LoginFailure::LaunchFailed));

        let other_environment = check_login(
            &host,
            AgentKind::Codex,
            &Environment::Windows,
            "/usr/bin/codex",
            LOGIN_CHECK_LIMIT,
        )
        .await;
        assert_eq!(other_environment, failed(LoginFailure::LaunchFailed));
    }
}
