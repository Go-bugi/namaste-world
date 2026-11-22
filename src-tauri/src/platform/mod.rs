mod fs;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod native;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod process;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::{
    app_dir, git_bash_available, hook_target, on_run_event, on_window_event, plugins,
    read_shell_env, show_over_full_screen, AgentCli, AgentHost, DETECT_AT_STARTUP,
};
#[cfg(target_os = "windows")]
pub use windows::{
    app_dir, git_bash_available, hook_target, read_shell_env, AgentCli, AgentHost,
    DETECT_AT_STARTUP,
};

/// 셸 환경 읽기에 주는 시간(SC-AGENT-003). 셸 설정 파일이 느리면 이 시간 뒤에 앱 환경으로 진행한다.
#[cfg(any(target_os = "macos", target_os = "windows"))]
const SHELL_ENV_LIMIT: std::time::Duration = std::time::Duration::from_secs(5);

/// 앱이 받은 환경 변수. 셸 환경을 읽지 못하면 이 값으로 CLI를 실행한다.
pub fn app_vars() -> std::collections::BTreeMap<String, String> {
    std::env::vars_os()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.to_string_lossy().into_owned(),
            )
        })
        .collect()
}

/// 앱이 받은 환경 그대로의 실행 환경. 아직 셸 환경을 읽지 않았을 때 쓴다.
pub fn app_env() -> crate::shell::model::ShellEnv {
    crate::shell::model::ShellEnv::new(app_vars())
}

/// 창 배치를 논리 좌표(포인트)로 계산하는가. macOS는 모니터 위치를 "논리 원점 × 그 모니터의 배율"로 주어
/// 배율이 다른 모니터끼리 물리 좌표가 겹치므로 논리 좌표로 계산한다. Windows는 전역 물리 좌표가 하나라 그대로 쓴다.
#[cfg(target_os = "macos")]
pub const LOGICAL_DESKTOP: bool = true;
#[cfg(not(target_os = "macos"))]
pub const LOGICAL_DESKTOP: bool = false;

/// macOS가 아니면 창 이벤트는 기본 동작을 따른다.
#[cfg(not(target_os = "macos"))]
pub fn on_window_event(_window: &tauri::Window, _event: &tauri::WindowEvent) {}

/// macOS가 아니면 데스크톱과 전체 화면을 따로 다루지 않는다. 항상 위 설정만으로 창 모드 전체 화면 위에 뜬다.
#[cfg(not(target_os = "macos"))]
pub fn show_over_full_screen(_window: &tauri::WebviewWindow) {}

/// macOS가 아니면 OS별 플러그인이 없다.
#[cfg(not(target_os = "macos"))]
pub fn plugins<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
}

/// macOS가 아니면 앱 이벤트에서 따로 할 일이 없다.
#[cfg(not(target_os = "macos"))]
pub fn on_run_event(_app: &tauri::AppHandle, _event: tauri::RunEvent) {}

pub use fs::write_file;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub use unsupported::{
    app_dir, git_bash_available, hook_target, read_shell_env, AgentCli, AgentHost,
    DETECT_AT_STARTUP,
};

/// macOS와 Windows가 아닌 OS. 확인할 환경이 없어 감지 결과가 비어 있다.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod unsupported {
    use crate::agent::domain::model::{EnvProbe, Environment, FailureReason, Target};
    use crate::agent::host::Host;
    use crate::shell::model::{ShellEnv, ShellEnvFailure, ShellKind};

    pub struct AgentHost;

    pub const DETECT_AT_STARTUP: bool = true;

    pub fn git_bash_available() -> bool {
        false
    }

    /// 읽을 셸이 없어 앱 환경을 그대로 쓴다.
    pub async fn read_shell_env(_shell: ShellKind) -> (ShellEnv, Option<ShellEnvFailure>) {
        (super::app_env(), None)
    }

    /// 설치 대상이 없으므로 쓰이지 않는다.
    pub type HookFs = super::fs::NativeFs;

    /// 앱 파일을 두는 폴더. `~/.namaste-world`
    pub fn app_dir() -> Option<std::path::PathBuf> {
        std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".namaste-world"))
    }

    /// 감지 항목이 없으므로 호출되지 않는다.
    pub async fn hook_target(
        _entry: &crate::agent::domain::model::AgentEntry,
        _version: &str,
        _app_dir: &std::path::Path,
        _env: &ShellEnv,
    ) -> Result<
        (crate::hook::domain::model::InstallTarget, HookFs),
        crate::hook::domain::model::InstallFailure,
    > {
        Err(crate::hook::domain::model::InstallFailure::Io {
            message: "지원하지 않는 OS입니다".to_owned(),
        })
    }

    impl AgentHost {
        pub fn new(_env: ShellEnv) -> Self {
            Self
        }
    }

    /// 조회할 CLI가 없어 늘 실패한다. 고정 목록이 쓰인다.
    pub struct AgentCli;

    impl AgentCli {
        pub fn new(_env: ShellEnv) -> Self {
            Self
        }
    }

    impl crate::model::host::ModelHost for AgentCli {
        async fn claude_init<T: Send>(
            &self,
            _environment: &Environment,
            _path: &str,
            _request: &str,
            _accept: impl Fn(&str) -> Option<T> + Send + Sync,
        ) -> Option<T> {
            None
        }

        async fn codex_models(&self, _environment: &Environment, _path: &str) -> Option<String> {
            None
        }

        async fn codex_config(&self, _environment: &Environment) -> Option<String> {
            None
        }
    }

    /// 실행할 CLI가 없어 늘 띄우지 못한 것으로 낸다.
    impl crate::login::host::LoginHost for AgentCli {
        async fn run(
            &self,
            _environment: &Environment,
            _path: &str,
            _args: &[&str],
        ) -> Option<crate::login::host::CliOutput> {
            None
        }
    }

    impl Host for AgentHost {
        async fn targets(&self) -> Vec<Target> {
            Vec::new()
        }

        async fn probe(&self, _environment: &Environment) -> EnvProbe {
            Err(FailureReason::EnvironmentError {
                message: "지원하지 않는 OS입니다".to_owned(),
            })
        }
    }
}
