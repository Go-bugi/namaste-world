use super::native::probe_native;
use super::process;
use crate::agent::detect::DETECTION_LIMIT;
use crate::agent::domain::model::{EnvProbe, Environment, FailureReason, HostOs, Target};
use crate::agent::domain::parse::{parse_distro_list, parse_probe_output};
use crate::agent::host::Host;
use crate::shell::choice::git_bash_candidates;
use crate::shell::env::{parse_env_json, reads_shell, shell_env_outcome, ShellRun};
use crate::shell::model::{ShellEnv, ShellEnvFailure, ShellKind};
use crate::shell::wsl::{after_fence, login_shell_script, shell_quote};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;

/// `wsl.exe -l`에 주는 시간.
const WSL_LIST_LIMIT: Duration = Duration::from_secs(3);
const PROBE_MARKER: &str = "__NAMASTE_AGENT__";
const ENV_MARKER: &str = "__NAMASTE_ENV__";
/// WSL 로그인 셸 설정 파일이 찍은 출력과 명령 출력을 가르는 줄.
const FENCE_MARKER: &str = "__NAMASTE_FENCE__";

// ---------------------------------------------------------------- 셸 환경

/// Windows는 셸 선택 단계에서 다음을 누를 때 감지한다(FR-AGENT-007, FR-START-013).
pub const DETECT_AT_STARTUP: bool = false;

/// 환경 변수 전체를 `<마커>{JSON}<마커>`로 찍는 PowerShell 스크립트. 큰따옴표와 `/`를 쓰지 않아
/// Git Bash가 Windows 프로그램에 인자를 넘길 때 바꾸지 않는다.
fn env_json_script() -> String {
    format!(
        "[Console]::OutputEncoding=[Text.Encoding]::UTF8; $e=@{{}}; Get-ChildItem Env: | ForEach-Object {{ $e[$_.Name]=$_.Value }}; [Console]::Out.Write('{ENV_MARKER}' + ($e | ConvertTo-Json -Compress) + '{ENV_MARKER}')"
    )
}

/// 고른 셸의 설정 파일을 읽은 환경(FR-AGENT-014). 명령 프롬프트는 셸을 띄우지 않고 앱 환경을 그대로 쓴다.
/// PowerShell은 7(`pwsh.exe`)을 먼저 띄워 보고, 띄우지 못하면 Windows PowerShell을 쓴다.
/// Git Bash는 대화형 로그인 셸 안에서 `powershell.exe -NoProfile`로 환경을 찍어 Windows 모양의 값을 읽는다.
pub async fn read_shell_env(shell: ShellKind) -> (ShellEnv, Option<ShellEnvFailure>) {
    let app = super::app_vars();
    let run = match shell {
        _ if !reads_shell(shell) => return (ShellEnv::new(app), None),
        ShellKind::GitBash => match git_bash_path() {
            Some(bash) => {
                let mut command = Command::new(bash);
                command.arg("-ilc").arg(format!(
                    "powershell.exe -NoLogo -NoProfile -NonInteractive -Command {}",
                    shell_quote(&env_json_script())
                ));
                process::shell_run(process::run_checked(command, super::SHELL_ENV_LIMIT).await)
            }
            None => ShellRun::LaunchFailed,
        },
        ShellKind::PowerShell | ShellKind::Cmd => {
            // 명령 프롬프트는 위에서 끝나므로 여기는 PowerShell이다
            let first = powershell_run("pwsh.exe").await;
            if first == ShellRun::LaunchFailed {
                powershell_run("powershell.exe").await
            } else {
                first
            }
        }
    };
    shell_env_outcome(
        &app,
        run,
        |stdout| parse_env_json(&String::from_utf8_lossy(stdout), ENV_MARKER),
        true,
    )
}

/// 프로필을 읽는 PowerShell로 환경을 찍는다.
async fn powershell_run(program: &str) -> ShellRun {
    let mut command = Command::new(program);
    command
        .args(["-NoLogo", "-NonInteractive", "-Command"])
        .arg(env_json_script());
    process::shell_run(process::run_checked(command, super::SHELL_ENV_LIMIT).await)
}

/// Git for Windows의 `bash.exe`. 흔한 설치 폴더와 `PATH`의 `Git\cmd`에서 찾는다.
fn git_bash_path() -> Option<PathBuf> {
    let var = |name: &str| std::env::var(name).ok();
    let program_files: Vec<String> = ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"]
        .into_iter()
        .filter_map(var)
        .collect();
    let dirs: Vec<&str> = program_files.iter().map(String::as_str).collect();
    let local = var("LOCALAPPDATA");
    let path_env = var("PATH").unwrap_or_default();
    git_bash_candidates(&dirs, local.as_deref(), &path_env)
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

/// Git for Windows가 설치되어 있으면 true(FR-START-011).
pub fn git_bash_available() -> bool {
    git_bash_path().is_some()
}

// ---------------------------------------------------------------- 에이전트 감지

/// Windows의 감지 대상. 환경은 Windows 자체와 WSL 배포판들이다.
pub struct AgentHost {
    env: ShellEnv,
    home: PathBuf,
    app_data: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
}

impl AgentHost {
    /// Windows 자체는 고른 셸의 환경(`env`)으로 확인한다. WSL은 배포판의 로그인 셸로 확인한다.
    pub fn new(env: ShellEnv) -> Self {
        let dir = |name: &str| std::env::var_os(name).map(PathBuf::from);
        Self {
            env,
            home: dir("USERPROFILE").unwrap_or_default(),
            app_data: dir("APPDATA"),
            local_app_data: dir("LOCALAPPDATA"),
        }
    }
}

impl Host for AgentHost {
    async fn targets(&self) -> Vec<Target> {
        let windows = Target {
            environment: Environment::Windows,
            running: true,
        };
        let Some(distros) = wsl_list(&["-l", "-q"]).await else {
            return vec![windows];
        };
        // 실행 중인 목록을 읽지 못하면 모두 멈춘 것으로 본다. 앱 시작 감지가 배포판을 켜지 않게 하기 위해서다
        let running = wsl_list(&["-l", "--running", "-q"])
            .await
            .unwrap_or_default();
        std::iter::once(windows)
            .chain(distros.into_iter().map(|distro| Target {
                running: running.contains(&distro),
                environment: Environment::Wsl { distro },
            }))
            .collect()
    }

    async fn probe(&self, environment: &Environment) -> EnvProbe {
        match environment {
            Environment::Windows => Ok(probe_native(
                HostOs::Windows,
                &self.env,
                &self.home,
                self.app_data.as_deref(),
                self.local_app_data.as_deref(),
            )
            .await),
            Environment::Wsl { distro } => probe_wsl(distro).await,
            Environment::Macos => Err(FailureReason::EnvironmentError {
                message: "Windows에서는 확인할 수 없는 환경입니다".to_owned(),
            }),
        }
    }
}

fn wsl_command() -> Command {
    let mut command = Command::new("wsl.exe");
    // wsl.exe 자신의 출력을 UTF-16 대신 UTF-8로 받는다
    command.env("WSL_UTF8", "1");
    command
}

/// `wsl.exe -l ...`의 배포판 이름 목록. WSL이 없거나 실패하면 `None`이다.
async fn wsl_list(args: &[&str]) -> Option<Vec<String>> {
    let mut command = wsl_command();
    command.args(args);
    let output = process::run(command, WSL_LIST_LIMIT).await?;
    output
        .status
        .success()
        .then(|| parse_distro_list(&output.stdout))
}

/// 배포판 사용자의 로그인 셸로 `inner`를 실행하는 명령(FR-AGENT-013). 셸 설정 파일이 찍은 출력은
/// `FENCE_MARKER` 줄 앞에 남으므로 출력은 `after_fence`로 읽는다.
fn wsl_login(distro: &str, inner: &str) -> Command {
    let mut command = wsl_command();
    command.args([
        "-d",
        distro,
        "--cd",
        "~",
        "-e",
        "sh",
        "-c",
        &login_shell_script(inner, FENCE_MARKER),
    ]);
    command
}

/// 성공한 출력에서 경계 마커 뒤의 stdout.
fn fenced_stdout(output: &std::process::Output) -> Option<String> {
    let stdout = process::stdout_text(output)?;
    after_fence(&stdout, FENCE_MARKER).map(str::to_owned)
}

/// 배포판 안에서 프로브 스크립트를 한 번 실행한다. 멈춘 배포판이면 이 실행이 배포판을 켠다.
async fn probe_wsl(distro: &str) -> EnvProbe {
    let command = wsl_login(distro, &probe_script());
    // 감지 전체의 시간 초과는 detect가 Timeout으로 처리하므로, 여기서 None은 대개 wsl.exe를 띄우지 못한 경우다
    let output = process::run(command, DETECTION_LIMIT)
        .await
        .ok_or_else(|| FailureReason::EnvironmentError {
            message: format!("WSL 배포판 {distro}을 실행하지 못했습니다"),
        })?;
    fenced_stdout(&output)
        .map(|stdout| parse_probe_output(&stdout, PROBE_MARKER))
        .ok_or_else(|| FailureReason::EnvironmentError {
            message: format!("WSL 배포판 {distro}에서 명령을 실행하지 못했습니다"),
        })
}

/// 배포판 안에서 실행 파일을 찾고 버전을 찍는 한 줄짜리 `sh` 스크립트.
///
/// - 로그인 셸이 `sh`라 `.bashrc`를 읽지 않는 배포판을 위해 흔한 설치 폴더를 `PATH` 앞에 붙인다.
/// - `/mnt/<드라이브>` 아래는 Windows 쪽 실행 파일이라 건너뛴다.
/// - Windows 명령줄 인자로 넘기므로 줄바꿈 없이 `;`로 잇는다.
fn probe_script() -> String {
    let commands = crate::agent::domain::model::AgentKind::ALL
        .map(|agent| agent.command())
        .join(" ");
    [
        "for d in \"$HOME/.local/bin\" \"$HOME/.npm-global/bin\" \"$HOME/.bun/bin\" \"$HOME/.volta/bin\" \"$HOME\"/.nvm/versions/node/*/bin; do [ -d \"$d\" ] && PATH=\"$d:$PATH\"; done",
        &format!("for c in {commands}; do p=''; IFS=:; for d in $PATH; do case \"$d\" in /mnt/[a-z]|/mnt/[a-z]/*) continue;; esac; if [ -f \"$d/$c\" ] && [ -x \"$d/$c\" ]; then p=\"$d/$c\"; break; fi; done; unset IFS"),
        "if [ -n \"$p\" ]; then v=$(timeout 3 \"$p\" --version 2>/dev/null </dev/null | head -n 1)",
        &format!("printf '%s\\t%s\\t%s\\t%s\\n' '{PROBE_MARKER}' \"$c\" \"$p\" \"$v\"; fi; done"),
    ]
    .join("; ")
}

// ---------------------------------------------------------------- 훅 설치

use super::fs::{blocking, read_file, write_file, NativeFs};
use crate::agent::domain::model::{AgentEntry, AgentKind};
use crate::hook::domain::model::{InstallFailure, InstallTarget};
use crate::hook::fs::ConfigFs;
use crate::shell::env::config_dir;
use std::io;
use std::path::Path;

/// 배포판 안의 명령(`printf`, `wslpath`, `chmod`)에 주는 시간.
const WSL_COMMAND_LIMIT: Duration = Duration::from_secs(5);
/// 배포판 로그인 셸로 설정 폴더를 읽는 데 주는 시간. 대화형 셸 설정 파일(nvm 등)과 배포판이 뜨는 시간을 더해 둔다.
const WSL_LOGIN_LIMIT: Duration = Duration::from_secs(10);

/// Windows의 설정 파일 접근. Windows 자체는 그대로, WSL 배포판은 `\\wsl$` 경로로 읽고 쓴다.
pub enum HookFs {
    Native(NativeFs),
    Wsl(WslFs),
}

impl ConfigFs for HookFs {
    async fn read(&self, path: &str) -> io::Result<Option<String>> {
        match self {
            HookFs::Native(fs) => fs.read(path).await,
            HookFs::Wsl(fs) => fs.read(path).await,
        }
    }

    async fn write(&self, path: &str, contents: &str, mode: u32) -> io::Result<()> {
        match self {
            HookFs::Native(fs) => fs.write(path, contents, mode).await,
            HookFs::Wsl(fs) => fs.write(path, contents, mode).await,
        }
    }
}

/// WSL 배포판 하나의 파일. 경로는 배포판 안의 POSIX 경로다.
pub struct WslFs {
    distro: String,
}

impl WslFs {
    fn host_path(&self, path: &str) -> PathBuf {
        PathBuf::from(format!(
            r"\\wsl$\{}{}",
            self.distro,
            path.replace('/', "\\")
        ))
    }
}

impl ConfigFs for WslFs {
    async fn read(&self, path: &str) -> io::Result<Option<String>> {
        let host = self.host_path(path);
        blocking(move || read_file(&host)).await
    }

    /// Windows 쪽에서는 배포판 안의 심볼릭 링크를 따라가지 못하고 POSIX 권한도 줄 수 없다.
    /// 그래서 링크와 기존 권한은 배포판 안에서 `readlink -f`, `stat`으로 읽고, 쓴 뒤 `chmod`로 권한을 맞춘다.
    /// 기존 파일은 원래 권한을, 새 파일은 `mode`를 준다.
    async fn write(&self, path: &str, contents: &str, mode: u32) -> io::Result<()> {
        let resolved = wsl_output(&self.distro, &["readlink", "-f", path])
            .await
            .unwrap_or_else(|_| path.to_owned());
        let existing_mode = wsl_output(&self.distro, &["stat", "-c", "%a", &resolved])
            .await
            .ok();
        let host = self.host_path(&resolved);
        let contents = contents.to_owned();
        blocking(move || write_file(&host, &contents, mode)).await?;
        let mode = existing_mode.unwrap_or_else(|| format!("{mode:o}"));
        let mut command = wsl_command();
        command.args(["-d", &self.distro, "-e", "chmod", &mode, &resolved]);
        match process::run(command, WSL_COMMAND_LIMIT).await {
            Some(output) if output.status.success() => Ok(()),
            _ => Err(io::Error::other(format!(
                "{resolved}의 권한을 맞추지 못했습니다"
            ))),
        }
    }
}

/// 앱 파일(엔드포인트, 훅 스크립트)을 두는 폴더. `%USERPROFILE%\.namaste-world`
pub fn app_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".namaste-world"))
}

/// 감지 항목의 설치 대상. WSL 배포판이면 그 배포판의 홈과 설정 폴더, 배포판에서 읽을 엔드포인트 경로를 알아낸다.
/// 설정 폴더는 그 환경에서 CLI를 실행하는 환경의 `CLAUDE_CONFIG_DIR`, `CODEX_HOME`을 따른다(FR-HOOK-002).
/// WSL은 배포판의 로그인 셸에서, Windows 자체는 고른 셸의 환경(`env`)에서 읽는다.
pub async fn hook_target(
    entry: &AgentEntry,
    version: &str,
    app_dir: &Path,
    env: &ShellEnv,
) -> Result<(InstallTarget, HookFs), InstallFailure> {
    let endpoint = app_dir.join("endpoint").to_string_lossy().into_owned();
    let target =
        |home: String, env: &ShellEnv, separator: char, endpoint_path: String| InstallTarget {
            agent: entry.agent,
            environment: entry.environment.clone(),
            version: version.to_owned(),
            claude_dir: config_dir(env, AgentKind::Claude, &home, separator),
            codex_dir: config_dir(env, AgentKind::Codex, &home, separator),
            home,
            endpoint_path,
        };
    match &entry.environment {
        Environment::Wsl { distro } => {
            let (home, wsl_env) = wsl_config_env(distro).await?;
            let endpoint_path = wsl_output(distro, &["wslpath", "-u", &endpoint]).await?;
            Ok((
                target(home, &wsl_env, '/', endpoint_path),
                HookFs::Wsl(WslFs {
                    distro: distro.clone(),
                }),
            ))
        }
        Environment::Windows | Environment::Macos => {
            let home = app_dir.parent().ok_or_else(|| InstallFailure::Io {
                message: "홈 폴더를 찾지 못했습니다".to_owned(),
            })?;
            Ok((
                target(home.to_string_lossy().into_owned(), env, '\\', endpoint),
                HookFs::Native(NativeFs),
            ))
        }
    }
}

/// 배포판 로그인 셸의 `HOME`과, 설정 폴더 변수만 담은 환경.
async fn wsl_config_env(distro: &str) -> Result<(String, ShellEnv), InstallFailure> {
    let command = wsl_login(
        distro,
        "printf '%s\\0' \"$HOME\" \"${CLAUDE_CONFIG_DIR-}\" \"${CODEX_HOME-}\"",
    );
    let failed = || InstallFailure::Io {
        message: format!("WSL 배포판 {distro}의 설정 폴더를 읽지 못했습니다"),
    };
    let output = process::run(command, WSL_LOGIN_LIMIT)
        .await
        .ok_or_else(failed)?;
    let stdout = fenced_stdout(&output).ok_or_else(failed)?;
    // 값에 줄바꿈이 있어도 밀리지 않게 NUL로 나눈다
    let mut lines = stdout.split('\0');
    let home = lines
        .next()
        .filter(|home| !home.is_empty())
        .ok_or_else(failed)?
        .to_owned();
    let vars = ["CLAUDE_CONFIG_DIR", "CODEX_HOME"]
        .into_iter()
        .zip(lines)
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect();
    Ok((home, ShellEnv::new(vars)))
}

/// 배포판 안에서 명령을 실행하고 stdout 첫 줄을 돌려준다.
async fn wsl_output(distro: &str, args: &[&str]) -> Result<String, InstallFailure> {
    let mut command = wsl_command();
    command.args(["-d", distro, "-e"]).args(args);
    process::run(command, WSL_COMMAND_LIMIT)
        .await
        .as_ref()
        .and_then(process::stdout_text)
        .and_then(|text| text.lines().next().map(str::to_owned))
        .filter(|line| !line.is_empty())
        .ok_or_else(|| InstallFailure::Io {
            message: format!("WSL 배포판 {distro}에서 {}을 실행하지 못했습니다", args[0]),
        })
}

// ---------------------------------------------------------------- 모델 목록 조회

use crate::model::host::{ModelHost, CLAUDE_INIT_ARGS, CLAUDE_INIT_SETTINGS, CODEX_MODELS_ARGS};
use crate::model::query::MODEL_QUERY_LIMIT;

/// Windows의 모델 목록 조회와 로그인 확인. Windows 자체는 감지한 실행 파일을 고른 셸의 환경으로,
/// WSL 배포판은 그 배포판의 로그인 셸로 실행한다(FR-AGENT-013, FR-AGENT-014, FR-AGENT-015).
/// 환경은 `Command::envs`로만 넘긴다.
pub struct AgentCli {
    env: ShellEnv,
    home: PathBuf,
}

impl AgentCli {
    pub fn new(env: ShellEnv) -> Self {
        Self {
            env,
            home: std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .unwrap_or_default(),
        }
    }

    /// Windows 자체에서 `path`를 실행하는 명령.
    fn native(&self, path: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut command = Command::new(path);
        command
            .env_clear()
            .envs(self.env.vars())
            .current_dir(&self.home);
        command
    }

    /// 배포판 안에서 로그인 셸로 `path args...`를 실행하는 명령. 감지 프로브처럼 흔한 설치 폴더를 `PATH` 앞에 붙여 `node`를 찾게 한다.
    fn wsl_exec(distro: &str, path: &str, args: &[&str]) -> Command {
        let argv = std::iter::once(path)
            .chain(args.iter().copied())
            .map(shell_quote)
            .collect::<Vec<_>>()
            .join(" ");
        // wsl.exe를 죽여도 배포판 안의 프로세스는 남을 수 있어 coreutils timeout으로 한 번 더 상한을 건다
        let inner = format!(
            "for d in \"$HOME/.local/bin\" \"$HOME/.npm-global/bin\" \"$HOME/.bun/bin\" \"$HOME/.volta/bin\" \"$HOME\"/.nvm/versions/node/*/bin; do [ -d \"$d\" ] && PATH=\"$d:$PATH\"; done; if command -v timeout >/dev/null 2>&1; then exec timeout -s KILL 6 {argv}; fi; exec {argv}"
        );
        wsl_login(distro, &inner)
    }
}

impl ModelHost for AgentCli {
    async fn claude_init<T: Send>(
        &self,
        environment: &Environment,
        path: &str,
        request: &str,
        accept: impl Fn(&str) -> Option<T> + Send + Sync,
    ) -> Option<T> {
        let command = match environment {
            Environment::Wsl { distro } => {
                let args: Vec<&str> = CLAUDE_INIT_ARGS
                    .iter()
                    .copied()
                    .chain([CLAUDE_INIT_SETTINGS])
                    .collect();
                Self::wsl_exec(distro, path, &args)
            }
            Environment::Windows | Environment::Macos => {
                // npm이 만든 claude.cmd는 배치 파일이라 따옴표와 중괄호가 든 인자를 넘기면 실행이 거부된다.
                // 그래서 설정은 조회마다 이름이 다른 임시 파일로 쓰고 그 경로를 넘긴다. 겹친 조회가 서로의 파일을 덮지 않게 한다
                static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let settings = std::env::temp_dir().join(format!(
                    "namaste-world-claude-settings-{}-{n}.json",
                    std::process::id()
                ));
                tokio::fs::write(&settings, CLAUDE_INIT_SETTINGS)
                    .await
                    .ok()?;
                let mut command = self.native(path);
                command.args(CLAUDE_INIT_ARGS).arg(&settings);
                let models = process::request_line(command, request, accept).await;
                let _ = tokio::fs::remove_file(&settings).await;
                return models;
            }
        };
        process::request_line(command, request, accept).await
    }

    async fn codex_models(&self, environment: &Environment, path: &str) -> Option<String> {
        match environment {
            Environment::Wsl { distro } => {
                let command = Self::wsl_exec(distro, path, &CODEX_MODELS_ARGS);
                let output = process::run_tree(command, MODEL_QUERY_LIMIT).await?;
                fenced_stdout(&output)
            }
            Environment::Windows | Environment::Macos => {
                let mut command = self.native(path);
                command.args(CODEX_MODELS_ARGS);
                let output = process::run_tree(command, MODEL_QUERY_LIMIT).await?;
                process::stdout_text(&output)
            }
        }
    }

    async fn codex_config(&self, environment: &Environment) -> Option<String> {
        match environment {
            Environment::Wsl { distro } => {
                // FR-HOOK-002의 설정 폴더 규칙과 같다. 비어 있는 CODEX_HOME은 없는 것으로 본다
                let command = wsl_login(distro, "cat \"${CODEX_HOME:-$HOME/.codex}/config.toml\"");
                let output = process::run(command, WSL_COMMAND_LIMIT).await?;
                fenced_stdout(&output)
            }
            Environment::Windows | Environment::Macos => {
                let home = self.home.to_string_lossy();
                let dir = config_dir(&self.env, AgentKind::Codex, &home, '\\');
                tokio::fs::read_to_string(Path::new(&dir).join("config.toml"))
                    .await
                    .ok()
            }
        }
    }
}

// ---------------------------------------------------------------- 로그인 상태 확인

use crate::login::check::LOGIN_CHECK_LIMIT;
use crate::login::host::{CliOutput, LoginHost};

impl LoginHost for AgentCli {
    async fn run(&self, environment: &Environment, path: &str, args: &[&str]) -> Option<CliOutput> {
        match environment {
            Environment::Wsl { distro } => {
                let command = Self::wsl_exec(distro, path, args);
                let output = process::run_tree_capture(command, LOGIN_CHECK_LIMIT).await?;
                // 경계 마커가 없으면 셸이 CLI까지 가지 못한 것이라 띄우지 못한 것으로 본다
                let CliOutput { stdout, stderr } = process::cli_output(&output);
                Some(CliOutput {
                    stdout: after_fence(&stdout, FENCE_MARKER)?.to_owned(),
                    stderr: after_fence(&stderr, FENCE_MARKER)?.to_owned(),
                })
            }
            Environment::Windows | Environment::Macos => {
                let mut command = self.native(path);
                command.args(args);
                let output = process::run_tree_capture(command, LOGIN_CHECK_LIMIT).await?;
                Some(process::cli_output(&output))
            }
        }
    }
}
