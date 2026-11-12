use tauri::Manager;

/// 창을 닫아도 앱은 종료하지 않고 창만 숨긴다. 종료는 Cmd+Q로 한다.
pub fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
}

/// 독 아이콘을 클릭했을 때 보이는 창이 없으면 숨긴 창을 다시 띄운다.
pub fn on_run_event(app: &tauri::AppHandle, event: tauri::RunEvent) {
    if let tauri::RunEvent::Reopen {
        has_visible_windows: false,
        ..
    } = event
    {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

// ---------------------------------------------------------------- 에이전트 감지

use super::native::probe_native;
use super::process;
use crate::agent::domain::model::{EnvProbe, Environment, FailureReason, HostOs, Target, Trigger};
use crate::agent::domain::parse::parse_marked_path;
use crate::agent::host::Host;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tokio::process::Command;

/// 로그인 셸에서 `PATH`를 읽는 데 주는 시간. 셸 설정 파일이 느리면 이 시간 뒤에 포기한다.
const SHELL_PATH_LIMIT: Duration = Duration::from_secs(3);
const PATH_MARKER: &str = "__NAMASTE_PATH__";

/// 앱이 켜져 있는 동안 재사용하는 로그인 셸 `PATH`. 다시 감지할 때만 새로 읽는다.
static SHELL_PATH: Mutex<Option<String>> = Mutex::new(None);

/// macOS의 감지 대상. 환경은 macOS 하나다.
pub struct AgentHost {
    path_env: String,
    home: PathBuf,
}

impl AgentHost {
    /// Finder나 Dock으로 띄운 앱은 셸의 `PATH`를 물려받지 못하므로 로그인 셸의 `PATH`를 앞에 붙인다.
    pub async fn new(trigger: Trigger) -> Self {
        let cached = match trigger {
            Trigger::Startup => SHELL_PATH.lock().ok().and_then(|path| path.clone()),
            Trigger::UserRequested => None,
        };
        let shell_path = match cached {
            Some(path) => Some(path),
            None => {
                let path = read_login_shell_path().await;
                if let (Some(path), Ok(mut slot)) = (&path, SHELL_PATH.lock()) {
                    *slot = Some(path.clone());
                }
                path
            }
        };
        let app_path = std::env::var("PATH").unwrap_or_default();
        Self {
            path_env: shell_path.map_or(app_path.clone(), |path| format!("{path}:{app_path}")),
            home: std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default(),
        }
    }
}

impl Host for AgentHost {
    async fn targets(&self) -> Vec<Target> {
        vec![Target {
            environment: Environment::Macos,
            running: true,
        }]
    }

    async fn probe(&self, environment: &Environment) -> EnvProbe {
        match environment {
            Environment::Macos => {
                Ok(probe_native(HostOs::Macos, &self.path_env, &self.home, None, None).await)
            }
            Environment::Windows | Environment::Wsl { .. } => {
                Err(FailureReason::EnvironmentError {
                    message: "macOS에서는 확인할 수 없는 환경입니다".to_owned(),
                })
            }
        }
    }
}

/// `$SHELL -ilc`로 셸 설정 파일을 읽힌 뒤 마커로 감싼 `PATH`를 찍어 읽는다.
/// 새 프로세스 그룹으로 띄워 대화형 셸이 앱을 띄운 터미널을 차지하지 못하게 한다.
async fn read_login_shell_path() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_owned());
    let mut command = Command::new(shell);
    command
        .arg("-ilc")
        .arg(format!(
            "printf '%s%s%s' {PATH_MARKER} \"$PATH\" {PATH_MARKER}"
        ))
        .process_group(0);
    let output = process::run(command, SHELL_PATH_LIMIT).await?;
    parse_marked_path(&process::stdout_text(&output)?, PATH_MARKER)
}
