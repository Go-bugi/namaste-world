/// 창을 닫아도 앱은 종료하지 않고 창만 숨긴다. 종료는 Cmd+Q로 한다.
pub fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
}

/// 독 아이콘을 클릭했을 때 보이는 창이 없으면 지금 쓰는 창(작은 창이나 시작 설정 창)을 다시 띄운다.
pub fn on_run_event(app: &tauri::AppHandle, event: tauri::RunEvent) {
    if let tauri::RunEvent::Reopen {
        has_visible_windows: false,
        ..
    } = event
    {
        crate::window::show_current(app);
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

// ---------------------------------------------------------------- 훅 설치

use super::fs::NativeFs;
use crate::agent::domain::model::AgentEntry;
use crate::hook::domain::model::{InstallFailure, InstallTarget};
use std::path::Path;

/// macOS의 설정 파일 접근.
pub type HookFs = NativeFs;

/// 앱 파일(엔드포인트, 훅 스크립트)을 두는 폴더. `~/.namaste-world`
pub fn app_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".namaste-world"))
}

/// 감지 항목의 설치 대상. macOS 환경 하나뿐이다.
pub async fn hook_target(
    entry: &AgentEntry,
    version: &str,
    app_dir: &Path,
) -> Result<(InstallTarget, HookFs), InstallFailure> {
    let home = app_dir.parent().ok_or_else(|| InstallFailure::Io {
        message: "홈 폴더를 찾지 못했습니다".to_owned(),
    })?;
    Ok((
        InstallTarget {
            agent: entry.agent,
            environment: entry.environment.clone(),
            version: version.to_owned(),
            home: home.to_string_lossy().into_owned(),
            endpoint_path: app_dir.join("endpoint").to_string_lossy().into_owned(),
        },
        NativeFs,
    ))
}

// ---------------------------------------------------------------- 모델 목록 조회

use crate::model::host::{ModelHost, CLAUDE_INIT_ARGS, CLAUDE_INIT_SETTINGS, CODEX_MODELS_ARGS};

/// macOS의 모델 목록 조회. 감지와 같은 로그인 셸 `PATH`로 실행해 npm으로 설치한 CLI가 `node`를 찾게 한다.
pub struct ModelCli {
    path_env: String,
    home: PathBuf,
}

impl ModelCli {
    /// 감지 때 읽어 둔 로그인 셸 `PATH`를 쓴다. 아직 없으면 셸을 다시 띄우지 않고 앱의 `PATH`만 쓴다.
    /// 셸을 띄우면 조회 시간 상한(SC-START-001) 밖에서 몇 초가 더 걸릴 수 있기 때문이다.
    pub async fn new() -> Self {
        let app_path = std::env::var("PATH").unwrap_or_default();
        let shell_path = SHELL_PATH.lock().ok().and_then(|path| path.clone());
        Self {
            path_env: shell_path.map_or(app_path.clone(), |path| format!("{path}:{app_path}")),
            home: std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default(),
        }
    }

    fn command(&self, path: &str) -> Command {
        let mut command = Command::new(path);
        command.env("PATH", &self.path_env).current_dir(&self.home);
        command
    }
}

impl ModelHost for ModelCli {
    async fn claude_init<T: Send>(
        &self,
        _environment: &Environment,
        path: &str,
        request: &str,
        accept: impl Fn(&str) -> Option<T> + Send + Sync,
    ) -> Option<T> {
        let mut command = self.command(path);
        command.args(CLAUDE_INIT_ARGS).arg(CLAUDE_INIT_SETTINGS);
        process::request_line(command, request, accept).await
    }

    async fn codex_models(&self, _environment: &Environment, path: &str) -> Option<String> {
        let mut command = self.command(path);
        command.args(CODEX_MODELS_ARGS);
        let output = process::run_tree(command, crate::model::query::MODEL_QUERY_LIMIT).await?;
        process::stdout_text(&output)
    }

    async fn codex_config(&self, _environment: &Environment) -> Option<String> {
        tokio::fs::read_to_string(self.home.join(".codex").join("config.toml"))
            .await
            .ok()
    }
}
