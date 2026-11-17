use crate::agent::domain::model::Detection;
use crate::hook::domain::model::HookInstallation;
use crate::model::domain::model::Selection;
use crate::shell::model::{ShellEnv, ShellKind};
use std::sync::Mutex;
use tokio::sync::watch;

/// 마지막으로 끝난 감지 결과. 앱 시작 감지가 끝나기 전에는 `None`이다.
/// 디스크에 저장하지 않으므로 앱을 다시 실행하면 새로 감지한다.
pub struct AgentDetection {
    latest: watch::Sender<Option<Detection>>,
}

impl AgentDetection {
    pub fn new() -> Self {
        Self {
            latest: watch::Sender::new(None),
        }
    }

    /// 보관한 결과. 아직 없으면 첫 결과가 들어올 때까지 기다린다.
    /// 다시 감지하는 중에도 끝나기 전까지는 이전 결과를 바로 준다.
    pub async fn latest(&self) -> Option<Detection> {
        let mut receiver = self.latest.subscribe();
        let result = receiver.wait_for(Option::is_some).await;
        result.ok().and_then(|value| value.clone())
    }

    /// 끝난 감지 결과로 보관한 결과를 바꾼다.
    pub fn publish(&self, detection: Detection) {
        self.latest.send_replace(Some(detection));
    }
}

/// 마지막으로 끝난 훅 설치 결과. 첫 설치가 끝나기 전에는 `None`이다.
pub struct HookInstallState {
    latest: watch::Sender<Option<HookInstallation>>,
}

impl HookInstallState {
    pub fn new() -> Self {
        Self {
            latest: watch::Sender::new(None),
        }
    }

    /// 보관한 결과. 아직 없으면 첫 결과가 들어올 때까지 기다린다.
    pub async fn latest(&self) -> Option<HookInstallation> {
        let mut receiver = self.latest.subscribe();
        let result = receiver.wait_for(Option::is_some).await;
        result.ok().and_then(|value| value.clone())
    }

    /// 끝난 설치 결과로 보관한 결과를 바꾼다.
    pub fn publish(&self, installation: HookInstallation) {
        self.latest.send_replace(Some(installation));
    }
}

/// 시작 설정에서 고른 에이전트와 모델. 이번 실행에서 다음을 누르기 전에는 `None`이다.
/// 파일 저장은 명령이 맡고, 여기에는 이번 실행에서 고른 값만 둔다.
pub struct SelectionState {
    current: Mutex<Option<Selection>>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }

    /// 이번 실행에서 고른 값.
    pub fn get(&self) -> Option<Selection> {
        self.current.lock().ok().and_then(|current| current.clone())
    }

    /// 고른 값을 바꾼다.
    pub fn set(&self, selection: Selection) {
        if let Ok(mut current) = self.current.lock() {
            *current = Some(selection);
        }
    }
}

/// 마지막으로 읽은 셸 환경과 그 셸. 감지가 끝날 때만 바뀐다. 디스크에 쓰지 않는다(FR-AGENT-015, FR-AGENT-017).
pub struct ShellEnvState {
    current: Mutex<Option<(ShellKind, ShellEnv)>>,
}

impl ShellEnvState {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }

    /// 감지에 쓴 셸과 환경으로 바꾼다.
    pub fn set(&self, shell: ShellKind, env: ShellEnv) {
        if let Ok(mut current) = self.current.lock() {
            *current = Some((shell, env));
        }
    }

    /// 마지막으로 읽은 환경. 아직 없으면 `app`(앱이 받은 환경)을 준다. 셸을 띄우지 않는다.
    pub fn env_or(&self, app: impl FnOnce() -> ShellEnv) -> ShellEnv {
        let read = self
            .current
            .lock()
            .ok()
            .and_then(|current| current.as_ref().map(|(_, env)| env.clone()));
        read.unwrap_or_else(app)
    }

    /// 마지막 감지에 쓴 셸. 아직 감지하지 않았으면 None.
    pub fn shell(&self) -> Option<ShellKind> {
        self.current
            .lock()
            .ok()
            .and_then(|current| current.as_ref().map(|(shell, _)| *shell))
    }
}

/// 창 전환을 한 번에 하나만 하게 하는 잠금. 다음 버튼을 연달아 눌러도 작은 창이 하나만 열린다.
pub struct WindowSwitch(pub tokio::sync::Mutex<()>);

impl WindowSwitch {
    pub fn new() -> Self {
        Self(tokio::sync::Mutex::new(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::domain::model::{AgentEntry, AgentKind, AgentStatus, Environment};
    use std::time::Duration;
    use tokio::time::timeout;

    fn detection(codex: AgentStatus) -> Detection {
        Detection {
            entries: vec![
                AgentEntry {
                    agent: AgentKind::Claude,
                    environment: Environment::Macos,
                    status: AgentStatus::NotInstalled,
                },
                AgentEntry {
                    agent: AgentKind::Codex,
                    environment: Environment::Macos,
                    status: codex,
                },
            ],
            shell_env_failure: None,
        }
    }

    fn before_install() -> Detection {
        detection(AgentStatus::NotInstalled)
    }

    fn after_install() -> Detection {
        detection(AgentStatus::Installed {
            path: "/opt/homebrew/bin/codex".to_string(),
            version: "0.146.0".to_string(),
        })
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_007_stored_result_is_returned_right_away() {
        let state = AgentDetection::new();
        state.publish(before_install());

        let got = timeout(Duration::ZERO, state.latest()).await;

        assert_eq!(got, Ok(Some(before_install())));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_007_request_before_startup_detection_waits() {
        let state = AgentDetection::new();

        let got = timeout(Duration::from_secs(60), state.latest()).await;

        assert!(got.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_007_waiting_request_gets_startup_result() {
        let state = AgentDetection::new();

        let (got, ()) = tokio::join!(state.latest(), async {
            tokio::time::sleep(Duration::from_secs(3)).await;
            state.publish(before_install());
        });

        assert_eq!(got, Some(before_install()));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_007_redetect_result_replaces_stored_result() {
        let state = AgentDetection::new();
        state.publish(before_install());
        state.publish(after_install());

        assert_eq!(state.latest().await, Some(after_install()));
        assert_eq!(state.latest().await, Some(after_install()));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_007_request_during_redetect_gets_previous_result_right_away() {
        let state = AgentDetection::new();
        state.publish(before_install());

        // 다시 감지는 끝날 때만 publish하므로, 진행 중에는 보관한 결과가 바뀌지 않는다
        let (got, ()) = tokio::join!(timeout(Duration::ZERO, state.latest()), async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            state.publish(after_install());
        });

        assert_eq!(got, Ok(Some(before_install())));
        assert_eq!(state.latest().await, Some(after_install()));
    }
}

#[cfg(test)]
mod hook_install_state_tests {
    use super::*;
    use crate::agent::domain::model::{AgentKind, Environment};
    use crate::hook::domain::model::{InstallEntry, InstallFailure, InstallStatus};
    use std::time::Duration;
    use tokio::time::timeout;

    fn installation(codex: InstallStatus) -> HookInstallation {
        HookInstallation {
            entries: vec![
                InstallEntry {
                    agent: AgentKind::Claude,
                    environment: Environment::Macos,
                    status: InstallStatus::Installed,
                },
                InstallEntry {
                    agent: AgentKind::Codex,
                    environment: Environment::Macos,
                    status: codex,
                },
            ],
        }
    }

    fn first_install() -> HookInstallation {
        installation(InstallStatus::Failed {
            reason: InstallFailure::NotWritable {
                file: "/Users/tester/.codex/hooks.json".to_string(),
            },
        })
    }

    fn after_redetect() -> HookInstallation {
        installation(InstallStatus::Installed)
    }

    #[tokio::test(start_paused = true)]
    async fn fr_hook_007_stored_result_is_returned_right_away() {
        let state = HookInstallState::new();
        state.publish(first_install());

        let got = timeout(Duration::ZERO, state.latest()).await;

        assert_eq!(got, Ok(Some(first_install())));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_hook_007_request_before_first_install_waits() {
        let state = HookInstallState::new();

        let got = timeout(Duration::from_secs(60), state.latest()).await;

        assert!(got.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn fr_hook_007_waiting_request_gets_first_install_result() {
        let state = HookInstallState::new();

        let (got, ()) = tokio::join!(state.latest(), async {
            tokio::time::sleep(Duration::from_secs(3)).await;
            state.publish(first_install());
        });

        assert_eq!(got, Some(first_install()));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_hook_007_later_install_result_replaces_stored_result() {
        let state = HookInstallState::new();
        state.publish(first_install());
        state.publish(after_redetect());

        assert_eq!(state.latest().await, Some(after_redetect()));
        assert_eq!(state.latest().await, Some(after_redetect()));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_hook_007_request_during_reinstall_gets_previous_result_right_away() {
        let state = HookInstallState::new();
        state.publish(first_install());

        let (got, ()) = tokio::join!(timeout(Duration::ZERO, state.latest()), async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            state.publish(after_redetect());
        });

        assert_eq!(got, Ok(Some(first_install())));
        assert_eq!(state.latest().await, Some(after_redetect()));
    }
}

#[cfg(test)]
mod selection_state_tests {
    use super::*;
    use crate::agent::domain::model::{AgentKind, Environment};

    fn selection(model: &str) -> Selection {
        Selection {
            agent: AgentKind::Claude,
            environment: Environment::Macos,
            model: model.to_owned(),
        }
    }

    #[test]
    fn fr_start_006_nothing_before_next() {
        assert_eq!(SelectionState::new().get(), None);
    }

    #[test]
    fn fr_start_006_window_reads_value_set_on_next() {
        let state = SelectionState::new();
        state.set(selection("sonnet"));
        state.set(selection("opus"));

        assert_eq!(state.get(), Some(selection("opus")));
    }
}

#[cfg(test)]
mod shell_env_state_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn env(pairs: &[(&str, &str)]) -> ShellEnv {
        ShellEnv::new(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    fn app_env() -> ShellEnv {
        env(&[("PATH", "/usr/bin:/bin")])
    }

    #[test]
    fn fr_agent_015_no_shell_before_first_detection() {
        assert_eq!(ShellEnvState::new().shell(), None);
    }

    #[test]
    fn fr_agent_015_without_read_env_uses_app_env() {
        let state = ShellEnvState::new();

        assert_eq!(state.env_or(app_env), app_env());
    }

    #[test]
    fn fr_agent_015_scenario_2_after_detection_uses_read_env_without_launching_shell() {
        let state = ShellEnvState::new();
        let read = env(&[
            ("PATH", "/opt/homebrew/bin:/usr/bin"),
            ("CLAUDE_CONFIG_DIR", "/w"),
        ]);
        state.set(ShellKind::PowerShell, read.clone());

        let got = state.env_or(|| panic!("읽은 환경이 있으면 앱 환경을 만들지 않는다"));

        assert_eq!(got, read);
        assert_eq!(state.shell(), Some(ShellKind::PowerShell));
    }

    #[test]
    fn fr_agent_015_scenario_1_redetection_replaces_env_and_shell() {
        let state = ShellEnvState::new();
        state.set(ShellKind::PowerShell, env(&[("PATH", "/usr/bin")]));
        let after = env(&[
            ("PATH", "/usr/bin"),
            ("CLAUDE_CONFIG_DIR", "/Users/a/work-claude"),
        ]);

        state.set(ShellKind::GitBash, after.clone());

        assert_eq!(state.env_or(app_env), after);
        assert_eq!(state.shell(), Some(ShellKind::GitBash));
    }
}
