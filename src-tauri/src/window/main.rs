//! 작은 창(`main`). 테두리 없이 투명하고 항상 위에 뜨며, 시작 설정 창이 있던 모니터의 오른쪽 아래 구석에 연다.
//! 모니터 구성이 바뀌거나 다시 불러낼 때 화면을 벗어나 있으면 주 모니터 구석으로 옮긴다.

use crate::placement::{corner_rect, pick_display, relocation, Display, Rect};
use crate::platform::{self, LOGICAL_DESKTOP};
use tauri::{
    LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

/// 작은 창의 label.
pub const LABEL: &str = "main";
/// 작은 창의 논리 크기.
const SIZE: (f64, f64) = (320.0, 320.0);
/// 작업 영역 가장자리와 띄우는 논리 여백.
const MARGIN: f64 = 16.0;

/// 작은 창을 만든다. 이미 있으면 그 창을 보이고 앞으로 가져온다.
/// `from`은 시작 설정 창이다. 그 창이 있던 모니터에 연다.
pub fn open(app: &tauri::AppHandle, from: Option<&WebviewWindow>) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = keep_visible(app);
        window.show()?;
        window.set_focus()?;
        return Ok(window);
    }
    // 숨긴 채 만들어 자리를 잡은 뒤 보인다. 배율이 다른 모니터로 옮기면서 크기가 바뀌는 모습이 보이지 않게 한다
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Namaste World")
        .inner_size(SIZE.0, SIZE.1)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .resizable(false)
        .skip_taskbar(false)
        .visible(false)
        .build()?;
    platform::show_over_full_screen(&window);
    // 자리를 잡다 실패하면 만든 창을 없앤다. 숨은 작은 창이 남으면 다시 불러낼 때 시작 설정 창 대신 그 창이 보인다
    let shown = target_rect(app, from)
        .map_or(Ok(()), |rect| place(&window, rect))
        .and_then(|()| window.show())
        .and_then(|()| window.set_focus());
    match shown {
        Ok(()) => Ok(window),
        Err(e) => {
            let _ = window.destroy();
            Err(e)
        }
    }
}

/// 작은 창이 화면을 벗어났으면 주 모니터 구석으로 옮긴다(FR-WINDOW-006, FR-WINDOW-010).
/// 작은 창이 없거나 최소화되어 있으면 아무것도 하지 않는다. 최소화된 창은 Windows에서 화면 밖 좌표를 돌려주기 때문이다.
/// 모니터, 주 모니터, 창 위치를 읽지 못하면 Err다. 확인하지 못한 것이므로 부르는 쪽이 다시 시도할 수 있다.
pub fn keep_visible(app: &tauri::AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    if window.is_minimized()? {
        return Ok(());
    }
    let (displays, primary) = desktop(app).ok_or_else(|| unread("모니터 목록"))?;
    // 목록과 주 모니터를 따로 읽어 그 사이 구성이 바뀌면 주 모니터를 찾지 못한다. 엉뚱한 모니터로 옮기지 않고 다시 시도하게 한다
    let primary = primary.ok_or_else(|| unread("주 모니터"))?;
    let current = window_rect(&window).ok_or_else(|| unread("작은 창 위치"))?;
    relocation(&displays, current, Some(primary), SIZE, MARGIN)
        .map_or(Ok(()), |rect| place(&window, rect))
}

/// 읽지 못한 값을 알리는 에러.
fn unread(what: &str) -> tauri::Error {
    std::io::Error::other(format!("{what}을(를) 읽지 못했습니다")).into()
}

/// 작은 창을 둘 사각형. 좌표계는 `display_of`와 같다. 모니터 정보를 읽지 못하면 None이고, 그때는 OS가 정한 자리에 연다.
fn target_rect(app: &tauri::AppHandle, from: Option<&WebviewWindow>) -> Option<Rect> {
    let (displays, primary) = desktop(app)?;
    // 시작 설정 창이 없으면 주 모니터를 고르도록 어느 모니터에도 속하지 않는 점을 넘긴다
    let source = from.and_then(window_rect).unwrap_or(Rect {
        x: i32::MIN / 2,
        y: i32::MIN / 2,
        width: 0,
        height: 0,
    });
    let index = pick_display(&displays, source, primary)?;
    Some(corner_rect(&displays[index], SIZE, MARGIN))
}

/// 모든 모니터와 주 모니터 번호. 모니터 목록을 읽지 못하면 None이다.
fn desktop(app: &tauri::AppHandle) -> Option<(Vec<Display>, Option<usize>)> {
    let monitors = app.available_monitors().ok()?;
    let displays = monitors.iter().map(display_of).collect();
    let primary = app.primary_monitor().ok().flatten().and_then(|primary| {
        monitors
            .iter()
            .position(|monitor| monitor.position() == primary.position())
    });
    Some((displays, primary))
}

/// 모니터 구성. 배치 계산과 같은 좌표계라 값이 달라지면 구성이 바뀐 것이다. 읽지 못하면 None이다.
pub fn displays(app: &tauri::AppHandle) -> Option<Vec<Display>> {
    desktop(app).map(|(displays, _)| displays)
}

/// 창의 바깥 사각형. 좌표계는 `display_of`와 같다.
fn window_rect(window: &WebviewWindow) -> Option<Rect> {
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    let scale = if LOGICAL_DESKTOP {
        window.scale_factor().ok()?
    } else {
        1.0
    };
    Some(scaled_rect(
        position.x,
        position.y,
        size.width,
        size.height,
        scale,
    ))
}

/// 모니터 하나를 배치 계산에 쓰는 좌표계로 바꾼다.
/// 논리 좌표계면 모니터마다 자기 배율로 나누고 배율은 1로 둔다. 물리 좌표계면 그대로 두고 배율을 함께 넘긴다.
fn display_of(monitor: &tauri::Monitor) -> Display {
    let scale = monitor.scale_factor();
    let divisor = if LOGICAL_DESKTOP { scale } else { 1.0 };
    let rect = |position: &PhysicalPosition<i32>, size: &PhysicalSize<u32>| {
        scaled_rect(position.x, position.y, size.width, size.height, divisor)
    };
    let work_area = monitor.work_area();
    Display {
        bounds: rect(monitor.position(), monitor.size()),
        work_area: rect(&work_area.position, &work_area.size),
        scale: if LOGICAL_DESKTOP { 1.0 } else { scale },
    }
}

/// 물리 픽셀 값을 `divisor`로 나눈 사각형. divisor가 1이면 그대로다.
fn scaled_rect(x: i32, y: i32, width: u32, height: u32, divisor: f64) -> Rect {
    let scale = |value: f64| (value / divisor).round();
    Rect {
        x: scale(f64::from(x)) as i32,
        y: scale(f64::from(y)) as i32,
        width: scale(f64::from(width)) as u32,
        height: scale(f64::from(height)) as u32,
    }
}

/// 창을 계산한 사각형에 맞춘다. 좌표계는 `display_of`와 같다.
/// macOS는 논리 좌표를 그대로 넘겨, 숨긴 창이 있는 화면의 배율로 다시 환산되지 않게 한다.
/// Windows는 먼저 그 모니터로 옮긴 뒤 크기를 정하고 다시 위치를 맞춘다. 배율이 다른 모니터로 옮기면 창 크기가 그 배율에 맞춰 바뀌기 때문이다.
fn place(window: &WebviewWindow, rect: Rect) -> tauri::Result<()> {
    if LOGICAL_DESKTOP {
        window.set_position(LogicalPosition::new(f64::from(rect.x), f64::from(rect.y)))?;
        return window.set_size(LogicalSize::new(
            f64::from(rect.width),
            f64::from(rect.height),
        ));
    }
    let position = PhysicalPosition::new(rect.x, rect.y);
    window.set_position(position)?;
    window.set_size(PhysicalSize::new(rect.width, rect.height))?;
    window.set_position(position)
}
