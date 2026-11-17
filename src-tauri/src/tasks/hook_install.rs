use crate::agent::domain::model::{AgentStatus, Detection};
use crate::events::HOOK_INSTALLATION;
use crate::hook::domain::command::should_install;
use crate::hook::domain::model::{
    HookInstallation, InstallEntry, InstallFailure, InstallStatus, ShellFamily,
};
use crate::hook::install::install;
use crate::platform;
use crate::shell::model::ShellEnv;
use crate::state::{AgentDetection, HookInstallState, ShellEnvState};
use futures_util::future::join_all;
use tauri::{Emitter, Manager};

/// 감지 결과에서 설치됨인 항목마다 훅을 설치한다. 대상끼리는 동시에 설치해 한 대상의 실패나 지연이 다른 대상을 막지 않는다.
/// 설정 폴더는 감지 때 읽은 CLI 실행 환경(`env`)으로 정한다(FR-HOOK-002).
pub async fn install_all(detection: &Detection, env: &ShellEnv) -> HookInstallation {
    let app_dir = platform::app_dir();
    let jobs = detection
        .entries
        .iter()
        .filter(|entry| should_install(entry))
        .map(|entry| {
            let app_dir = app_dir.clone();
            async move {
                let version = match &entry.status {
                    AgentStatus::Installed { version, .. } => version.as_str(),
                    _ => "",
                };
                let status = match app_dir {
                    None => InstallStatus::Failed {
                        reason: InstallFailure::Io {
                            message: "홈 폴더를 찾지 못했습니다".to_owned(),
                        },
                    },
                    Some(dir) => match platform::hook_target(entry, version, &dir, env).await {
                        Ok((target, fs)) => {
                            install(&fs, &target, ShellFamily::of(&target.environment)).await
                        }
                        Err(reason) => InstallStatus::Failed { reason },
                    },
                };
                InstallEntry {
                    agent: entry.agent,
                    environment: entry.environment.clone(),
                    status,
                }
            }
        });
    HookInstallation {
        entries: join_all(jobs).await,
    }
}

/// 설치를 한 번에 하나만 돌린다. 시작 설치 중에 다시 감지가 와도 같은 설정 파일을 동시에 고치지 않고,
/// 나중에 시작한 설치의 결과가 마지막으로 보관된다.
static INSTALL_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 설치하고 결과를 보관한 뒤 renderer에 알린다.
pub async fn run(app: &tauri::AppHandle, detection: &Detection) {
    let _guard = INSTALL_LOCK.lock().await;
    let env = app.state::<ShellEnvState>().env_or(platform::app_env);
    let installation = install_all(detection, &env).await;
    app.state::<HookInstallState>()
        .publish(installation.clone());
    let _ = app.emit(HOOK_INSTALLATION, installation);
}

/// 앱 시작 감지가 끝나기를 기다려 설치한다.
pub fn spawn_after_startup_detection(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Some(detection) = app.state::<AgentDetection>().latest().await {
            run(&app, &detection).await;
        }
    });
}
