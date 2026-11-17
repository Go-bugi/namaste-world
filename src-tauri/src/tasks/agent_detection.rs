use crate::agent::detect::{detect, DETECTION_LIMIT};
use crate::agent::domain::model::{Detection, Trigger};
use crate::platform::{self, AgentHost};
use crate::shell::env::remaining_limit;
use crate::shell::model::{ShellEnv, ShellKind};
use crate::state::{AgentDetection, ShellEnvState};
use tauri::Manager;
use tokio::time::Instant;

/// 고른 셸(macOS는 로그인 셸)의 환경을 읽고 그 환경으로 이 OS의 환경을 감지한다.
/// 셸 환경을 읽는 데 쓴 시간을 상한에서 빼서 전체가 상한 안에 끝난다(SC-AGENT-001, SC-AGENT-003).
/// 셸 환경을 읽지 못하면 앱 환경으로 감지하고 그 이유를 결과에 담는다(FR-AGENT-016, FR-AGENT-018).
pub async fn run(trigger: Trigger, shell: ShellKind) -> (Detection, ShellEnv) {
    let started = Instant::now();
    let (env, failure) = platform::read_shell_env(shell).await;
    let limit = remaining_limit(DETECTION_LIMIT, started.elapsed());
    let host = AgentHost::new(env.clone());
    let detection = detect(&host, trigger, limit).await;
    (
        Detection {
            shell_env_failure: failure,
            ..detection
        },
        env,
    )
}

/// 감지하고, 읽은 셸 환경과 감지 결과를 보관한다. 모델 조회와 로그인 확인, 훅 설치는 이 환경을 다시 쓴다(FR-AGENT-015).
pub async fn run_and_store(
    app: &tauri::AppHandle,
    trigger: Trigger,
    shell: ShellKind,
) -> Detection {
    // 감지끼리 겹쳐 서로 다른 셸의 환경과 결과가 섞여 보관되지 않게 한 번에 하나만 돌린다
    let _guard = DETECT_LOCK.lock().await;
    let (detection, env) = run(trigger, shell).await;
    app.state::<ShellEnvState>().set(shell, env);
    app.state::<AgentDetection>().publish(detection.clone());
    detection
}

static DETECT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 앱 시작 때 감지를 한 번 돌려 결과를 상태에 넣는다. 멈춘 WSL 배포판은 켜지 않는다.
/// 셸을 고르는 OS(Windows)는 셸 선택 단계에서 감지하므로 여기서 띄우지 않는다(FR-AGENT-007).
pub fn spawn_startup_detection(app: tauri::AppHandle) {
    if !platform::DETECT_AT_STARTUP {
        return;
    }
    tauri::async_runtime::spawn(async move {
        // macOS는 셸 선택이 없어 이 값을 쓰지 않는다
        run_and_store(&app, Trigger::Startup, ShellKind::PowerShell).await;
    });
}
