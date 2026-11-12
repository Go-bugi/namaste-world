#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod native;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod process;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::{on_run_event, on_window_event, AgentHost};
#[cfg(target_os = "windows")]
pub use windows::AgentHost;

/// macOS가 아니면 창 이벤트는 기본 동작을 따른다.
#[cfg(not(target_os = "macos"))]
pub fn on_window_event(_window: &tauri::Window, _event: &tauri::WindowEvent) {}

/// macOS가 아니면 앱 이벤트에서 따로 할 일이 없다.
#[cfg(not(target_os = "macos"))]
pub fn on_run_event(_app: &tauri::AppHandle, _event: tauri::RunEvent) {}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub use unsupported::AgentHost;

/// macOS와 Windows가 아닌 OS. 확인할 환경이 없어 감지 결과가 비어 있다.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod unsupported {
    use crate::agent::domain::model::{EnvProbe, Environment, FailureReason, Target, Trigger};
    use crate::agent::host::Host;

    pub struct AgentHost;

    impl AgentHost {
        pub async fn new(_trigger: Trigger) -> Self {
            Self
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
