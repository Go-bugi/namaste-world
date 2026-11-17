use crate::agent::domain::model::{Detection, Trigger};
use crate::platform;
use crate::shell::choice::{effective_shell, parse_shell_choice, shell_choice_json, shell_options};
use crate::shell::model::{ShellKind, ShellSetup};
use crate::state::{AgentDetection, ShellEnvState};
use crate::tasks::{agent_detection, hook_install};
use tauri::Manager;

/// 고른 셸을 저장하는 파일 이름. 앱 데이터 폴더 안에 둔다. 셸 환경 값은 저장하지 않는다(FR-AGENT-017).
const SHELL_FILE: &str = "shell.json";

/// 셸 선택 단계에 보여 줄 선택지와 미리 고를 셸. 셸을 고르지 않는 OS(macOS)는 None이다.
/// 저장한 Git Bash가 지금 없으면 PowerShell을 고른다(FR-START-012).
#[tauri::command]
pub async fn shell_setup(app: tauri::AppHandle) -> Result<Option<ShellSetup>, ()> {
    if platform::DETECT_AT_STARTUP {
        return Ok(None);
    }
    let git_bash = platform::git_bash_available();
    Ok(Some(ShellSetup {
        options: shell_options(git_bash),
        selected: effective_shell(saved_shell(&app).await, git_bash),
    }))
}

/// 고른 셸을 바로 저장한다. 감지는 하지 않는다(FR-START-012, FR-START-013).
/// 저장하지 못해도 이번 실행에는 지장이 없으므로 실패로 알리지 않는다.
#[tauri::command]
pub async fn set_shell(shell: ShellKind, app: tauri::AppHandle) -> Result<(), ()> {
    if let Ok(dir) = app.path().app_data_dir() {
        if tokio::fs::create_dir_all(&dir).await.is_ok() {
            let _ = tokio::fs::write(dir.join(SHELL_FILE), shell_choice_json(shell)).await;
        }
    }
    Ok(())
}

/// 셸 선택 단계의 다음 버튼(FR-START-013). 처음 부르면 앱 시작 감지로 보관하고, 훅 설치는 시작 설치 작업이 이어서 한다.
/// 마지막 감지와 셸이 같으면 보관한 결과를 준다. 다르면 다시 감지하고 훅을 설치한다.
/// 셸을 바꿔 하는 감지도 앱 시작 감지처럼 멈춘 WSL 배포판을 켜지 않는다.
#[tauri::command]
pub async fn detect_with_shell(
    shell: ShellKind,
    app: tauri::AppHandle,
    detection: tauri::State<'_, AgentDetection>,
    shell_env: tauri::State<'_, ShellEnvState>,
) -> Result<Detection, ()> {
    // 다음을 연달아 눌러도 감지가 겹치지 않게 한다
    static DETECT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = DETECT_LOCK.lock().await;
    let last = shell_env.shell();
    // 셸을 고르지 않는 OS(macOS)는 앱 시작 감지 결과를 그대로 준다
    if platform::DETECT_AT_STARTUP || last == Some(shell) {
        return detection.latest().await.ok_or(());
    }
    let result = agent_detection::run_and_store(&app, Trigger::Startup, shell).await;
    if last.is_some() {
        let installed = result.clone();
        tauri::async_runtime::spawn(async move { hook_install::run(&app, &installed).await });
    }
    Ok(result)
}

/// 앱 데이터 폴더에 저장한 셸. 없거나 읽지 못하면 None.
pub async fn saved_shell(app: &tauri::AppHandle) -> Option<ShellKind> {
    let dir = app.path().app_data_dir().ok()?;
    let contents = tokio::fs::read_to_string(dir.join(SHELL_FILE)).await.ok()?;
    parse_shell_choice(&contents)
}
