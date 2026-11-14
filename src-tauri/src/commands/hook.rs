use crate::hook::domain::model::HookInstallation;
use crate::state::HookInstallState;

/// 보관한 훅 설치 결과를 준다. 첫 설치가 아직이면 끝날 때까지 기다린다.
/// `State`를 받는 async 명령은 `Result`를 돌려줘야 해서 에러 타입을 `()`로 둔다. 실제로는 실패하지 않는다.
#[tauri::command]
pub async fn hook_installation(
    state: tauri::State<'_, HookInstallState>,
) -> Result<HookInstallation, ()> {
    state.latest().await.ok_or(())
}
