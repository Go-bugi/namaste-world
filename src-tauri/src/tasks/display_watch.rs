//! 모니터 구성을 지켜보다가 바뀌면 작은 창이 화면 안에 있는지 확인한다(FR-WINDOW-010).

use crate::window::main;
use std::time::Duration;
use tauri::Manager;

/// 모니터 구성을 읽는 간격. 구성이 바뀐 뒤 2초 안에 창을 옮기도록(SC-WINDOW-002) 그보다 충분히 짧게 둔다.
const DISPLAY_POLL: Duration = Duration::from_millis(500);

/// 모니터 구성을 `DISPLAY_POLL`마다 읽어, 앞 값과 다르고 작은 창이 보이는 중이면 위치를 확인한다.
/// 숨은 작은 창은 다시 불러낼 때 확인하므로 여기서는 건너뛴다. 확인에 실패하면 다음 주기에 다시 한다.
pub fn spawn(app: tauri::AppHandle) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(DISPLAY_POLL);
        let mut last = main::displays(&app);
        loop {
            interval.tick().await;
            let current = main::displays(&app);
            if current.is_none() || current == last {
                continue;
            }
            let visible = app
                .get_webview_window(main::LABEL)
                .and_then(|window| window.is_visible().ok())
                .unwrap_or(false);
            // 확인하지 못했으면 구성을 기억하지 않아 다음 주기에 다시 확인한다
            if !visible || main::keep_visible(&app).is_ok() {
                last = current;
            }
        }
    })
}
