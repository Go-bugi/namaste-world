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
pub use macos::{app_dir, hook_target, on_run_event, on_window_event, AgentCli, AgentHost};
#[cfg(target_os = "windows")]
pub use windows::{app_dir, hook_target, AgentCli, AgentHost};

/// 창 배치를 논리 좌표(포인트)로 계산하는가. macOS는 모니터 위치를 "논리 원점 × 그 모니터의 배율"로 주어
/// 배율이 다른 모니터끼리 물리 좌표가 겹치므로 논리 좌표로 계산한다. Windows는 전역 물리 좌표가 하나라 그대로 쓴다.
#[cfg(target_os = "macos")]
pub const LOGICAL_DESKTOP: bool = true;
#[cfg(not(target_os = "macos"))]
pub const LOGICAL_DESKTOP: bool = false;

/// macOS가 아니면 창 이벤트는 기본 동작을 따른다.
#[cfg(not(target_os = "macos"))]
pub fn on_window_event(_window: &tauri::Window, _event: &tauri::WindowEvent) {}

/// macOS가 아니면 앱 이벤트에서 따로 할 일이 없다.
#[cfg(not(target_os = "macos"))]
pub fn on_run_event(_app: &tauri::AppHandle, _event: tauri::RunEvent) {}

pub use fs::write_file;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub use unsupported::{app_dir, hook_target, AgentCli, AgentHost};

/// macOS와 Windows가 아닌 OS. 확인할 환경이 없어 감지 결과가 비어 있다.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod unsupported {
    use crate::agent::domain::model::{EnvProbe, Environment, FailureReason, Target, Trigger};
    use crate::agent::host::Host;

    pub struct AgentHost;

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
    ) -> Result<
        (crate::hook::domain::model::InstallTarget, HookFs),
        crate::hook::domain::model::InstallFailure,
    > {
        Err(crate::hook::domain::model::InstallFailure::Io {
            message: "지원하지 않는 OS입니다".to_owned(),
        })
    }

    impl AgentHost {
        pub async fn new(_trigger: Trigger) -> Self {
            Self
        }
    }

    /// 조회할 CLI가 없어 늘 실패한다. 고정 목록이 쓰인다.
    pub struct AgentCli;

    impl AgentCli {
        pub async fn new() -> Self {
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
