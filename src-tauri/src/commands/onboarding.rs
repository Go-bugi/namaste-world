use crate::error::OnboardingError;
use crate::model::domain::catalog::{parse_selection, selection_json};
use crate::model::domain::model::Selection;
use crate::state::{SelectionState, WindowSwitch};
use crate::window::{self, ONBOARDING};
use tauri::Manager;

/// 고른 값을 저장하는 파일 이름. 앱 데이터 폴더 안에 둔다.
const SELECTION_FILE: &str = "selection.json";

/// 이번 실행에서 고른 값, 없으면 지난번에 저장한 값. 둘 다 없거나 파일을 읽지 못하면 None이다.
#[tauri::command]
pub async fn saved_selection(
    app: tauri::AppHandle,
    state: tauri::State<'_, SelectionState>,
) -> Result<Option<Selection>, ()> {
    if let Some(selection) = state.get() {
        return Ok(Some(selection));
    }
    let Ok(dir) = app.path().app_data_dir() else {
        return Ok(None);
    };
    let contents = tokio::fs::read_to_string(dir.join(SELECTION_FILE))
        .await
        .ok();
    Ok(contents.as_deref().and_then(parse_selection))
}

/// 다음 버튼. 작은 창을 연 뒤 고른 값을 보관하고 저장하고, 시작 설정 창을 닫는다.
/// 파일 저장이 실패해도 이번 실행에는 지장이 없으므로 계속한다. 작은 창을 열지 못하면 값을 보관하지 않고 시작 설정 창도 닫지 않는다.
#[tauri::command]
pub async fn finish_onboarding(
    selection: Selection,
    app: tauri::AppHandle,
    state: tauri::State<'_, SelectionState>,
    switch: tauri::State<'_, WindowSwitch>,
) -> Result<(), OnboardingError> {
    let _guard = switch.0.lock().await;
    let onboarding = app.get_webview_window(ONBOARDING);
    window::main::open(&app, onboarding.as_ref()).map_err(|e| OnboardingError::WindowFailed {
        message: e.to_string(),
    })?;
    // 작은 창이 열린 뒤에만 고른 값으로 확정한다
    state.set(selection.clone());
    if let Ok(dir) = app.path().app_data_dir() {
        if tokio::fs::create_dir_all(&dir).await.is_ok() {
            let _ = tokio::fs::write(dir.join(SELECTION_FILE), selection_json(&selection)).await;
        }
    }
    if let Some(onboarding) = onboarding {
        // 닫기 요청을 숨기기로 바꾸는 창 이벤트를 거치지 않고 창을 없앤다
        let _ = onboarding.destroy();
    }
    Ok(())
}
