use crate::hook::domain::model::HookInstallation;
use crate::state::{AgentDetection, HookInstallState};
use crate::tasks::hook_install;

/// 보관한 훅 설치 결과를 준다. 첫 설치가 아직이면 끝날 때까지 기다린다.
/// `State`를 받는 async 명령은 `Result`를 돌려줘야 해서 에러 타입을 `()`로 둔다. 실제로는 실패하지 않는다.
#[tauri::command]
pub async fn hook_installation(
    state: tauri::State<'_, HookInstallState>,
) -> Result<HookInstallation, ()> {
    state.latest().await.ok_or(())
}

/// 보관한 감지 결과로 훅을 다시 설치하고, 보관한 설치 결과를 바꾼 뒤 그 결과를 준다.
/// 끝나면 앱 시작 설치처럼 `hook-installation` 이벤트도 보낸다.
#[tauri::command]
pub async fn reinstall_hooks(
    app: tauri::AppHandle,
    detection: tauri::State<'_, AgentDetection>,
    installation: tauri::State<'_, HookInstallState>,
) -> Result<HookInstallation, ()> {
    let detection = detection.latest().await.ok_or(())?;
    hook_install::run(&app, &detection).await;
    installation.latest().await.ok_or(())
}
