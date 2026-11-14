use super::native::probe_native;
use super::process;
use crate::agent::detect::DETECTION_LIMIT;
use crate::agent::domain::model::{EnvProbe, Environment, FailureReason, HostOs, Target, Trigger};
use crate::agent::domain::parse::{parse_distro_list, parse_probe_output};
use crate::agent::host::Host;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;

/// `wsl.exe -l`에 주는 시간.
const WSL_LIST_LIMIT: Duration = Duration::from_secs(3);
const PROBE_MARKER: &str = "__NAMASTE_AGENT__";

/// Windows의 감지 대상. 환경은 Windows 자체와 WSL 배포판들이다.
pub struct AgentHost {
    path_env: String,
    home: PathBuf,
    app_data: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
}

impl AgentHost {
    pub async fn new(_trigger: Trigger) -> Self {
        let dir = |name: &str| std::env::var_os(name).map(PathBuf::from);
        Self {
            path_env: std::env::var("PATH").unwrap_or_default(),
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
                &self.path_env,
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

/// 배포판 안에서 프로브 스크립트를 한 번 실행한다. 멈춘 배포판이면 이 실행이 배포판을 켠다.
async fn probe_wsl(distro: &str) -> EnvProbe {
    let mut command = wsl_command();
    command.args(["-d", distro, "-e", "sh", "-lc", &probe_script()]);
    // 감지 전체의 시간 초과는 detect가 Timeout으로 처리하므로, 여기서 None은 대개 wsl.exe를 띄우지 못한 경우다
    let output = process::run(command, DETECTION_LIMIT)
        .await
        .ok_or_else(|| FailureReason::EnvironmentError {
            message: format!("WSL 배포판 {distro}을 실행하지 못했습니다"),
        })?;
    process::stdout_text(&output)
        .map(|stdout| parse_probe_output(&stdout, PROBE_MARKER))
        .ok_or_else(|| FailureReason::EnvironmentError {
            message: format!("WSL 배포판 {distro}에서 명령을 실행하지 못했습니다"),
        })
}

/// 배포판 안에서 실행 파일을 찾고 버전을 찍는 한 줄짜리 `sh` 스크립트.
///
/// - `sh -lc`는 `.bashrc`를 읽지 않으므로 흔한 설치 폴더를 `PATH` 앞에 붙인다.
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
use crate::agent::domain::model::AgentEntry;
use crate::hook::domain::model::{InstallFailure, InstallTarget};
use crate::hook::fs::ConfigFs;
use std::io;
use std::path::Path;

/// 배포판 안의 명령(`printf`, `wslpath`, `chmod`)에 주는 시간.
const WSL_COMMAND_LIMIT: Duration = Duration::from_secs(5);

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

/// 감지 항목의 설치 대상. WSL 배포판이면 그 배포판의 홈과, 배포판에서 읽을 엔드포인트 경로를 알아낸다.
pub async fn hook_target(
    entry: &AgentEntry,
    version: &str,
    app_dir: &Path,
) -> Result<(InstallTarget, HookFs), InstallFailure> {
    let endpoint = app_dir.join("endpoint").to_string_lossy().into_owned();
    let target = |home: String, endpoint_path: String| InstallTarget {
        agent: entry.agent,
        environment: entry.environment.clone(),
        version: version.to_owned(),
        home,
        endpoint_path,
    };
    match &entry.environment {
        Environment::Wsl { distro } => {
            let home = wsl_output(distro, &["sh", "-c", "printf '%s' \"$HOME\""]).await?;
            let endpoint_path = wsl_output(distro, &["wslpath", "-u", &endpoint]).await?;
            Ok((
                target(home, endpoint_path),
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
                target(home.to_string_lossy().into_owned(), endpoint),
                HookFs::Native(NativeFs),
            ))
        }
    }
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
