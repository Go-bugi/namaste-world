//! 창을 어느 모니터의 어디에 둘지 정한다. IO가 없어 Tauri 없이 테스트한다.
//! 좌표는 모두 물리 픽셀이고, 배율은 모니터마다 다르다.

/// 물리 픽셀 사각형. `x`, `y`는 왼쪽 위 모서리다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 모니터 하나. `work_area`는 작업 표시줄, 독, 메뉴 막대를 뺀 영역이다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Display {
    pub bounds: Rect,
    pub work_area: Rect,
    pub scale: f64,
}

/// 창 사각형의 중심이 들어 있는 모니터의 번호. 없으면 primary(주 모니터 번호). primary도 None이면 0. displays가 비면 None.
pub fn pick_display(displays: &[Display], window: Rect, primary: Option<usize>) -> Option<usize> {
    if displays.is_empty() {
        return None;
    }
    let center_x = i64::from(window.x) + i64::from(window.width) / 2;
    let center_y = i64::from(window.y) + i64::from(window.height) / 2;
    displays
        .iter()
        .position(|display| contains(display.bounds, center_x, center_y))
        .or(primary.filter(|index| *index < displays.len()))
        .or(Some(0))
}

fn contains(rect: Rect, x: i64, y: i64) -> bool {
    let left = i64::from(rect.x);
    let top = i64::from(rect.y);
    (left..left + i64::from(rect.width)).contains(&x)
        && (top..top + i64::from(rect.height)).contains(&y)
}

/// 논리 크기 `size`의 창을 그 모니터 작업 영역의 오른쪽 아래에, 논리 여백 `margin`만큼 띄워 둘 물리 사각형.
/// 크기와 여백은 그 모니터 배율을 곱하고 반올림한다. 가로와 세로는 따로 정한다.
/// 한 축의 창 크기가 작업 영역보다 크면 그 축만 작업 영역 크기로 줄이고, 여백은 그 축에 남는 만큼만(0~margin×배율) 둔다.
/// 결과는 항상 작업 영역 안에 있다.
pub fn corner_rect(display: &Display, size: (f64, f64), margin: f64) -> Rect {
    let area = display.work_area;
    let (x, width) = far_edge(area.x, area.width, size.0, margin, display.scale);
    let (y, height) = far_edge(area.y, area.height, size.1, margin, display.scale);
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// 한 축에서 작업 영역 끝 쪽에 붙인 창의 시작 좌표와 길이.
fn far_edge(start: i32, length: u32, size: f64, margin: f64, scale: f64) -> (i32, u32) {
    let physical = |value: f64| (value * scale).round().max(0.0) as u64;
    let length = u64::from(length);
    let window = physical(size).min(length);
    let gap = physical(margin).min(length - window);
    let offset = length - window - gap;
    ((i64::from(start) + offset as i64) as i32, window as u32)
}

/// 앱을 다시 불러낼 때 보일 창. 작은 창이 있으면 "main", 없으면 "onboarding".
pub fn window_to_show(main_exists: bool) -> &'static str {
    if main_exists {
        "main"
    } else {
        "onboarding"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const SIZE: (f64, f64) = (320.0, 320.0);
    const MARGIN: f64 = 16.0;

    fn rect(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn display(bounds: Rect, work_area: Rect, scale: f64) -> Display {
        Display {
            bounds,
            work_area,
            scale,
        }
    }

    /// 주 모니터(왼쪽, 0번)와 보조 모니터(오른쪽, 1번)
    fn two_displays() -> Vec<Display> {
        vec![
            display(rect(0, 0, 1920, 1080), rect(0, 0, 1920, 1040), 1.0),
            display(rect(1920, 0, 2560, 1440), rect(1920, 0, 2560, 1400), 1.0),
        ]
    }

    #[test]
    fn fr_window_003_window_on_secondary_display_picks_it() {
        let window = rect(2400, 300, 720, 560);

        assert_eq!(pick_display(&two_displays(), window, Some(0)), Some(1));
    }

    #[test]
    fn fr_window_003_window_on_primary_display_picks_it() {
        let window = rect(600, 260, 720, 560);

        assert_eq!(pick_display(&two_displays(), window, Some(0)), Some(0));
    }

    #[test]
    fn fr_window_003_window_across_two_displays_picks_the_one_with_center() {
        // 중심 x = 1700 + 360 = 2060, 오른쪽 모니터 안
        let window = rect(1700, 200, 720, 560);

        assert_eq!(pick_display(&two_displays(), window, Some(0)), Some(1));
    }

    #[test]
    fn fr_window_003_center_outside_all_displays_uses_primary() {
        let window = rect(-5000, -5000, 720, 560);

        assert_eq!(pick_display(&two_displays(), window, Some(1)), Some(1));
    }

    #[test]
    fn fr_window_003_no_primary_known_uses_first_display() {
        let window = rect(-5000, -5000, 720, 560);

        assert_eq!(pick_display(&two_displays(), window, None), Some(0));
    }

    #[test]
    fn fr_window_003_no_displays_is_none() {
        assert_eq!(pick_display(&[], rect(0, 0, 720, 560), Some(0)), None);
    }

    #[test]
    fn fr_window_003_window_on_display_left_of_primary_picks_it() {
        let displays = vec![
            display(rect(0, 0, 1920, 1080), rect(0, 0, 1920, 1040), 1.0),
            display(rect(-1920, 0, 1920, 1080), rect(-1920, 0, 1920, 1040), 1.0),
        ];

        assert_eq!(
            pick_display(&displays, rect(-1200, 200, 720, 560), Some(0)),
            Some(1)
        );
    }

    #[test]
    fn fr_window_003_corner_is_bottom_right_of_work_area_with_margin() {
        let d = display(rect(0, 0, 1920, 1080), rect(0, 0, 1920, 1040), 1.0);

        // x = 1920 - 16 - 320, y = 1040 - 16 - 320
        assert_eq!(corner_rect(&d, SIZE, MARGIN), rect(1584, 704, 320, 320));
    }

    #[test]
    fn fr_window_003_corner_on_secondary_display_uses_its_work_area() {
        let d = two_displays()[1];

        // x = 1920 + 2560 - 16 - 320, y = 1400 - 16 - 320
        assert_eq!(corner_rect(&d, SIZE, MARGIN), rect(4144, 1064, 320, 320));
    }

    #[test]
    fn fr_window_005_taskbar_at_bottom_keeps_window_above_it() {
        // 작업 표시줄 48px이 아래에 있음
        let d = display(rect(0, 0, 1920, 1080), rect(0, 0, 1920, 1032), 1.0);

        let got = corner_rect(&d, SIZE, MARGIN);

        assert_eq!(got.y + got.height as i32, 1032 - 16);
    }

    #[test]
    fn fr_window_005_dock_on_right_keeps_window_left_of_it() {
        // 메뉴 막대 25px이 위에, 독 80px이 오른쪽에 있음
        let d = display(rect(0, 0, 1440, 900), rect(0, 25, 1360, 875), 1.0);

        let got = corner_rect(&d, SIZE, MARGIN);

        assert_eq!(got, rect(1360 - 16 - 320, 25 + 875 - 16 - 320, 320, 320));
    }

    #[test]
    fn fr_window_004_scale_multiplies_size_and_margin() {
        // 200%: 크기 640, 여백 32
        let d = display(rect(0, 0, 3840, 2160), rect(0, 50, 3840, 2010), 2.0);

        // x = 3840 - 32 - 640, y = 50 + 2010 - 32 - 640
        assert_eq!(corner_rect(&d, SIZE, MARGIN), rect(3168, 1388, 640, 640));
    }

    #[test]
    fn fr_window_004_fractional_scale() {
        // 150%: 크기 480, 여백 24
        let d = display(rect(1920, 0, 2880, 1620), rect(1920, 0, 2880, 1560), 1.5);

        // x = 1920 + 2880 - 24 - 480, y = 1560 - 24 - 480
        assert_eq!(corner_rect(&d, SIZE, MARGIN), rect(4296, 1056, 480, 480));
    }

    #[test]
    fn fr_window_005_window_larger_than_work_area_shrinks_to_it() {
        let d = display(rect(0, 0, 300, 300), rect(0, 20, 300, 260), 1.0);

        assert_eq!(corner_rect(&d, SIZE, MARGIN), rect(0, 20, 300, 260));
    }

    #[test]
    fn fr_window_005_only_height_too_large_shrinks_height_only() {
        let d = display(rect(0, 0, 1920, 300), rect(0, 0, 1920, 300), 1.0);

        // 가로는 그대로 여백 16, 세로는 작업 영역 높이에 여백 0
        assert_eq!(corner_rect(&d, SIZE, MARGIN), rect(1584, 0, 320, 300));
    }

    #[test]
    fn fr_window_005_only_width_too_large_shrinks_width_only() {
        let d = display(rect(100, 0, 300, 1080), rect(100, 0, 300, 1040), 1.0);

        // x = 100, 너비 300 / y = 1040 - 16 - 320
        assert_eq!(corner_rect(&d, SIZE, MARGIN), rect(100, 704, 300, 320));
    }

    #[test]
    fn fr_window_005_margin_keeps_only_remaining_space() {
        // 세로로 남는 공간은 330 - 320 = 10이라 여백도 10만 둔다
        let d = display(rect(0, 0, 1920, 330), rect(0, 0, 1920, 330), 1.0);

        assert_eq!(corner_rect(&d, SIZE, MARGIN), rect(1584, 0, 320, 320));
    }

    #[test]
    fn fr_window_004_scaled_margin_rounds_up_from_half() {
        // 110%: 크기 320 × 1.1 = 352, 여백 16 × 1.1 = 17.6 → 18
        let d = display(rect(0, 0, 1920, 1080), rect(0, 0, 1920, 1040), 1.1);

        // x = 1920 - 18 - 352, y = 1040 - 18 - 352
        assert_eq!(corner_rect(&d, SIZE, MARGIN), rect(1550, 670, 352, 352));
    }

    #[test]
    fn fr_window_004_scaled_margin_rounds_down_below_half() {
        // 115%: 크기 320 × 1.15 = 368, 여백 16 × 1.15 = 18.4 → 18
        let d = display(rect(0, 0, 1920, 1080), rect(0, 0, 1920, 1040), 1.15);

        // x = 1920 - 18 - 368, y = 1040 - 18 - 368
        assert_eq!(corner_rect(&d, SIZE, MARGIN), rect(1534, 654, 368, 368));
    }

    fn display_strategy() -> impl Strategy<Value = Display> {
        (
            -4000i32..4000,
            -4000i32..4000,
            1u32..4000,
            1u32..4000,
            prop_oneof![
                Just(1.0),
                Just(1.25),
                Just(1.5),
                Just(1.75),
                Just(2.0),
                Just(3.0)
            ],
        )
            .prop_map(|(x, y, width, height, scale)| {
                let area = rect(x, y, width, height);
                display(area, area, scale)
            })
    }

    proptest! {
        #[test]
        fn fr_window_005_result_is_always_inside_work_area(
            d in display_strategy(),
            width in 1.0f64..2000.0,
            height in 1.0f64..2000.0,
            margin in 0.0f64..64.0,
        ) {
            let got = corner_rect(&d, (width, height), margin);
            let wa = d.work_area;

            prop_assert!(got.x >= wa.x);
            prop_assert!(got.y >= wa.y);
            prop_assert!(got.x as i64 + got.width as i64 <= wa.x as i64 + wa.width as i64);
            prop_assert!(got.y as i64 + got.height as i64 <= wa.y as i64 + wa.height as i64);
        }

        #[test]
        fn fr_window_004_logical_size_is_same_on_any_scale(
            scale in prop_oneof![Just(1.0), Just(1.25), Just(1.5), Just(2.0), Just(3.0)],
            x in -4000i32..4000,
            y in -4000i32..4000,
        ) {
            let area = rect(x, y, 3840, 2160);
            let got = corner_rect(&display(area, area, scale), SIZE, MARGIN);

            prop_assert_eq!(got.width as f64 / scale, 320.0);
            prop_assert_eq!(got.height as f64 / scale, 320.0);
            prop_assert_eq!((x as i64 + 3840 - (got.x as i64 + got.width as i64)) as f64 / scale, 16.0);
        }
    }

    #[test]
    fn fr_window_006_small_window_exists_shows_main() {
        assert_eq!(window_to_show(true), "main");
    }

    #[test]
    fn fr_window_006_before_switch_shows_onboarding() {
        assert_eq!(window_to_show(false), "onboarding");
    }
}
