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

use tauri_nspanel::{tauri_panel, CollectionBehavior, PanelLevel, StyleMask, WebviewWindowExt};

tauri_panel! {
    panel!(SmallPanel {
        config: {
            // 키 창이 될 수 있어야 Cmd+W 같은 단축키를 받는다(FR-WINDOW-007)
            can_become_key_window: true,
            is_floating_panel: true
        }
    })
}

/// 작은 창을 포커스를 빼앗지 않는 floating 패널로 바꾸고, 모든 데스크톱과 다른 앱의 전체 화면 공간에 보이게 한다(FR-WINDOW-009).
/// 독 아이콘이 있는 앱의 보통 창은 다른 앱의 전체 화면 공간에 들어가지 못해 패널로 바꾼다.
/// 창 클래스를 바꾸는 일은 메인 스레드에서만 할 수 있어 메인 스레드로 넘긴다. 패널에 maximize, fullscreen을 부르면 앱이 죽는다.
pub fn show_over_full_screen(window: &tauri::WebviewWindow) {
    let target = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Ok(panel) = target.to_panel::<SmallPanel>() else {
            return;
        };
        panel.set_level(PanelLevel::Floating.value());
        let _ = panel.add_style_mask(StyleMask::empty().nonactivating_panel().into());
        panel.set_collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary()
                .into(),
        );
        panel.set_hides_on_deactivate(false);
    });
}

/// macOS 전용 플러그인을 붙인다. 작은 창을 패널로 바꾸는 tauri-nspanel이다.
pub fn plugins<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.plugin(tauri_nspanel::init())
}

// ---------------------------------------------------------------- 셸 환경

use super::process;
use crate::shell::env::{parse_env_nul, shell_env_outcome};
use crate::shell::model::{ShellEnv, ShellEnvFailure, ShellKind};
use tokio::process::Command;

const ENV_MARKER: &str = "__NAMASTE_ENV__";

/// macOS는 앱을 켜면 바로 감지한다. 고를 셸이 없기 때문이다(FR-AGENT-007).
pub const DETECT_AT_STARTUP: bool = true;

/// macOS에는 셸 선택이 없어 Git Bash를 찾지 않는다.
pub fn git_bash_available() -> bool {
    false
}

/// `$SHELL -ilc`로 셸 설정 파일을 읽힌 뒤 마커 사이에 `env -0`을 찍어 환경 전체를 읽는다(FR-AGENT-012).
/// Finder나 Dock으로 띄운 앱은 셸 설정 파일의 변수를 물려받지 못하기 때문이다. `shell`은 쓰지 않는다.
/// 새 프로세스 그룹으로 띄워 대화형 셸이 앱을 띄운 터미널을 차지하지 못하게 하고, 시간을 넘기면 그룹째 죽인다.
pub async fn read_shell_env(_shell: ShellKind) -> (ShellEnv, Option<ShellEnvFailure>) {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_owned());
    let mut command = Command::new(shell);
    command.arg("-ilc").arg(format!(
        "printf '\\000%s\\000' {ENV_MARKER}; env -0; printf '\\000%s\\000' {ENV_MARKER}"
    ));
    let run = process::run_checked(command, super::SHELL_ENV_LIMIT).await;
    shell_env_outcome(
        &super::app_vars(),
        process::shell_run(run),
        |stdout| parse_env_nul(stdout, ENV_MARKER),
        false,
    )
}

// ---------------------------------------------------------------- 에이전트 감지

use super::native::probe_native;
use crate::agent::domain::model::{EnvProbe, Environment, FailureReason, HostOs, Target};
use crate::agent::host::Host;
use std::path::PathBuf;

/// macOS의 감지 대상. 환경은 macOS 하나다.
pub struct AgentHost {
    env: ShellEnv,
    home: PathBuf,
}

impl AgentHost {
    /// 감지 때 읽은 셸 환경으로 실행 파일을 찾고 `--version`을 실행한다.
    pub fn new(env: ShellEnv) -> Self {
        Self {
            home: home_of(&env),
            env,
        }
    }
}

/// 실행 환경의 `HOME`. 없으면 앱의 `HOME`.
fn home_of(env: &ShellEnv) -> PathBuf {
    env.get("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_default()
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
                Ok(probe_native(HostOs::Macos, &self.env, &self.home, None, None).await)
            }
            Environment::Windows | Environment::Wsl { .. } => {
                Err(FailureReason::EnvironmentError {
                    message: "macOS에서는 확인할 수 없는 환경입니다".to_owned(),
                })
            }
        }
    }
}

// ---------------------------------------------------------------- 훅 설치

use super::fs::NativeFs;
use crate::agent::domain::model::{AgentEntry, AgentKind};
use crate::hook::domain::model::{InstallFailure, InstallTarget};
use crate::shell::env::config_dir;
use std::path::Path;

/// macOS의 설정 파일 접근.
pub type HookFs = NativeFs;

/// 앱 파일(엔드포인트, 훅 스크립트)을 두는 폴더. `~/.namaste-world`
pub fn app_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".namaste-world"))
}

/// 감지 항목의 설치 대상. macOS 환경 하나뿐이다. 설정 폴더는 CLI 실행 환경(`env`)에서 정한다(FR-HOOK-002).
pub async fn hook_target(
    entry: &AgentEntry,
    version: &str,
    app_dir: &Path,
    env: &ShellEnv,
) -> Result<(InstallTarget, HookFs), InstallFailure> {
    let home = app_dir
        .parent()
        .ok_or_else(|| InstallFailure::Io {
            message: "홈 폴더를 찾지 못했습니다".to_owned(),
        })?
        .to_string_lossy()
        .into_owned();
    Ok((
        InstallTarget {
            agent: entry.agent,
            environment: entry.environment.clone(),
            version: version.to_owned(),
            claude_dir: config_dir(env, AgentKind::Claude, &home, '/'),
            codex_dir: config_dir(env, AgentKind::Codex, &home, '/'),
            home,
            endpoint_path: app_dir.join("endpoint").to_string_lossy().into_owned(),
        },
        NativeFs,
    ))
}

// ---------------------------------------------------------------- 모델 목록 조회

use crate::model::host::{ModelHost, CLAUDE_INIT_ARGS, CLAUDE_INIT_SETTINGS, CODEX_MODELS_ARGS};

/// macOS의 모델 목록 조회와 로그인 확인. 감지 때 읽은 셸 환경으로 CLI를 실행한다(FR-AGENT-015).
/// 환경은 `Command::envs`로만 넘긴다. 인자로 넘기면 다른 프로세스가 프로세스 목록에서 볼 수 있기 때문이다.
pub struct AgentCli {
    env: ShellEnv,
    home: PathBuf,
}

impl AgentCli {
    pub fn new(env: ShellEnv) -> Self {
        Self {
            home: home_of(&env),
            env,
        }
    }

    fn command(&self, path: &str) -> Command {
        let mut command = Command::new(path);
        command
            .env_clear()
            .envs(self.env.vars())
            .current_dir(&self.home);
        command
    }
}

impl ModelHost for AgentCli {
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
        let home = self.home.to_string_lossy();
        let dir = config_dir(&self.env, AgentKind::Codex, &home, '/');
        tokio::fs::read_to_string(Path::new(&dir).join("config.toml"))
            .await
            .ok()
    }
}

// ---------------------------------------------------------------- 로그인 상태 확인

use crate::login::check::LOGIN_CHECK_LIMIT;
use crate::login::host::{CliOutput, LoginHost};

impl LoginHost for AgentCli {
    async fn run(
        &self,
        _environment: &Environment,
        path: &str,
        args: &[&str],
    ) -> Option<CliOutput> {
        let mut command = self.command(path);
        command.args(args);
        let output = process::run_tree_capture(command, LOGIN_CHECK_LIMIT).await?;
        Some(process::cli_output(&output))
    }
}
