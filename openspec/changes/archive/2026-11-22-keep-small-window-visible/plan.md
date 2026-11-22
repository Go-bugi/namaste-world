# Plan

spec-hash: b07b6cc45b047863

## 설계

### 층 배치

renderer는 바뀌지 않습니다. 모두 `src-tauri`입니다.

| 층 | 파일 | 하는 일 |
| --- | --- | --- |
| core | `src/placement.rs` | 작은 창을 옮길 자리 계산(`relocation`). IO와 `tauri` import 없음 |
| shell(창) | `src/window/main.rs` | 작은 창을 만들 때 모든 데스크톱에 보이게 설정. 지금 위치를 읽어 `relocation`에 넘기고 결과대로 옮기는 `keep_visible` |
| shell(창) | `src/window/mod.rs` | `show_current`가 작은 창을 보이기 전에 `keep_visible`을 부름 |
| shell(작업) | `src/tasks/display_watch.rs`(새 파일), `src/tasks/mod.rs` | 모니터 구성을 주기적으로 읽고, 바뀌면 `window::main::keep_visible`을 부름 |
| 앱 조립 | `src/lib.rs` | `setup`에서 `tasks::display_watch::spawn`. Builder에 `platform::plugins`로 OS별 플러그인을 붙임 |
| platform | `src/platform/macos.rs`, `src/platform/mod.rs` | macOS에서 작은 창을 포커스를 빼앗지 않는 패널(NSPanel)로 바꾸고, 모든 데스크톱과 전체 화면 공간에 보이게 함. 패널 플러그인 등록. 다른 OS는 아무것도 하지 않는 같은 이름 함수 |
| 빌드 설정 | `Cargo.toml`, `Cargo.lock` | macOS 대상 의존성 `tauri-nspanel` 2.1(crates.io, objc2 0.6과 objc2-app-kit 0.3을 써서 tao와 같은 버전). 앞서 넣은 `objc2`, `objc2-app-kit` 직접 의존성은 뺌 |

`#[cfg(target_os)]` 코드는 `platform/`에만 둡니다. 로직 모듈 `placement`는 shell이 부르는 방향으로만 의존합니다.

### 타입과 경계

없음. Rust 명령, 이벤트, ts-rs 타입, capability 권한이 바뀌지 않습니다.

### 시그니처

`src/placement.rs`에 하나를 추가합니다. 기존 `Rect`, `Display`, `pick_display`, `corner_rect`는 그대로 씁니다.

```rust
/// 작은 창이 보이는 화면을 벗어났으면 옮길 사각형, 벗어나지 않았으면 None.
///
/// - `window`: 지금 창의 사각형. 좌표계는 `displays`와 같다.
/// - 창 중심이 어느 모니터의 `work_area` 안에라도 있으면 None(옮기지 않음, FR-WINDOW-006, FR-WINDOW-010).
///   `pick_display`가 `bounds`를 보는 것과 달리 작업 영역을 본다.
/// - 아니면 주 모니터(`primary`)에 `corner_rect(display, size, margin)`를 적용한 사각형을 돌려준다.
///   `primary`가 None이거나 범위 밖이면 0번 모니터를 쓴다.
/// - `displays`가 비면 None(옮길 곳이 없음).
/// - 중심 계산은 `pick_display`와 같다: `x + width / 2`, `y + height / 2`를 i64로 계산하고, 작업 영역은 오른쪽과 아래 끝을 포함하지 않는다.
pub fn relocation(
    displays: &[Display],
    window: Rect,
    primary: Option<usize>,
    size: (f64, f64),
    margin: f64,
) -> Option<Rect>
```

shell 함수(테스트 대상 아님, 수동 확인):

```rust
// window/main.rs
/// 작은 창이 있으면 지금 위치를 읽어, 화면을 벗어났을 때만 주 모니터 구석으로 옮긴다.
/// 창이 없거나 최소화되어 있으면 아무것도 하지 않는다(Ok).
/// 모니터 목록, 주 모니터, 창 위치를 읽지 못하면 Err다. 부르는 쪽이 다시 시도할 수 있다.
pub fn keep_visible(app: &tauri::AppHandle) -> tauri::Result<()>

// platform/macos.rs, platform/mod.rs(다른 OS는 빈 함수)
/// 작은 창을 포커스를 빼앗지 않는 floating 패널로 바꾸고, 모든 데스크톱과 다른 앱의 전체 화면 공간에 보이게 한다(FR-WINDOW-009).
pub fn show_over_full_screen(window: &tauri::WebviewWindow)

// platform/macos.rs, platform/mod.rs(다른 OS는 builder를 그대로 돌려줌)
/// OS별 플러그인을 붙인다. macOS는 tauri-nspanel 플러그인이다.
pub fn plugins<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R>

// tasks/display_watch.rs
/// 모니터 구성을 DISPLAY_POLL마다 읽어, 앞 값과 다르고 작은 창이 보이는 중이면 keep_visible을 부른다.
/// keep_visible이 Err면 구성을 기억하지 않아 다음 주기에 다시 확인한다.
pub fn spawn(app: tauri::AppHandle)
```

### 결정

- **모든 데스크톱:** 빌더의 `.visible_on_all_workspaces(true)`로 macOS `CanJoinAllSpaces`를 켭니다. Windows에서는 tao가 아무것도 하지 않아 Q3(제외)과 맞습니다.
- **전체 화면 위:** 독 아이콘이 있는 일반 앱의 보통 NSWindow는 `FullScreenAuxiliary`를 켜도 다른 앱의 전체 화면 공간에 보이지 않았습니다(작업 1.1에서 확인). 그래서 `tauri-nspanel`로 작은 창을 NSPanel로 바꿉니다(`to_panel`). 패널에는 포커스를 빼앗지 않는 style mask(`nonactivating_panel`), collection behavior `can_join_all_spaces` + `full_screen_auxiliary`, 레벨 `PanelLevel::Floating`, `hides_on_deactivate(false)`를 줍니다. Floating 레벨이라 메뉴 막대와 알림은 가리지 않습니다(FR-WINDOW-009). 패널 클래스는 `can_become_key_window: true`로 두어 Cmd+W(FR-WINDOW-007)와 끌기(FR-WINDOW-008)가 그대로 동작하게 합니다. 패널에 `maximize`, `fullscreen`을 부르면 앱이 죽으므로 부르지 않습니다. 활성화 정책은 regular 그대로 두어 독 아이콘(FR-WINDOW-001)과 독 클릭으로 다시 부르기(FR-WINDOW-006)를 지킵니다. accessory 정책은 독 아이콘이 사라져 쓰지 않습니다. 직접 NSPanel 하위 클래스를 만드는 방법은 tao 창 클래스의 동작을 다시 짜야 해서 쓰지 않습니다.
- **모니터 변경 감지:** `tasks/display_watch.rs`가 `DISPLAY_POLL = 500ms`마다 `available_monitors()`를 읽어 `Vec<Display>`를 앞 값과 비교합니다. 변경을 알아채고 옮기기까지 최악 약 0.5초라 SC-WINDOW-002(2초)를 지킵니다. OS 알림(macOS `NSApplicationDidChangeScreenParametersNotification`, Windows `WM_DISPLAYCHANGE`)은 OS마다 따로 짜야 해서 쓰지 않습니다.
- **옮기는 조건:** 구성이 바뀌었을 때 한 번만 확인합니다. 바뀌지 않으면 사용자가 끈 자리를 건드리지 않습니다(끈 뒤 밀어 넣기는 Q2에서 고르지 않음). OS가 먼저 창을 남은 모니터로 옮겨 두었으면 `relocation`이 None이라 그대로 둡니다.
- **다시 불러낼 때:** `show_current`는 최소화를 먼저 풀고, 작은 창이면 `keep_visible`을 부른 뒤 `show`합니다. Windows에서 최소화된 창은 위치가 (-32000, -32000)으로 읽히기 때문입니다. 숨긴 창도 `outer_position`을 읽을 수 있습니다. 이미 있는 작은 창을 `open`이 다시 보일 때도 `keep_visible`을 부릅니다.
- **좌표 변환:** `target_rect`의 `display_of`, `scaled_rect`, `place`를 그대로 씁니다. 지금 창 사각형을 만드는 코드는 `target_rect`의 시작 설정 창 부분을 함수로 빼서 함께 씁니다.

### 위험

- [위험] tauri-nspanel의 전체 화면 예제는 accessory 정책(독 아이콘 숨김)과 함께 쓰여, regular 정책에서도 전체 화면 위에 보이는지는 확인되지 않음 → 작업 1.1에서 먼저 실제 앱으로 확인합니다. 안 보이면 구현을 멈추고, accessory 정책으로 바꾸고 FR-WINDOW-001, FR-WINDOW-006을 고칠지 사람에게 묻습니다.
- [위험] 패널로 바꾼 뒤 투명 배경, 끌기, Cmd+W 닫기, 독 클릭 다시 부르기가 달라질 수 있음 → 작업 1.2에서 FR-WINDOW-001, 007, 008의 시나리오를 다시 확인합니다.
- [위험] 모니터를 뺀 직후 OS가 아직 배치를 정리하지 않아 옛 좌표를 읽음 → 다음 주기(0.5초 뒤)에 구성이 또 달라 보이면 다시 확인하므로, 2초 안에 맞는 자리로 갑니다.
- [위험] `available_monitors()`가 메인 스레드로 넘어가 0.5초마다 메인 스레드 일을 만듦 → 모니터 목록 조회는 가벼워 렌더링에 영향이 없음을 작업 4.3에서 확인합니다.
- [위험] tauri-nspanel이 끌어오는 `objc2` 버전이 tao와 달라 두 벌 빌드됨 → `cargo tree -d`로 중복이 없는지 봅니다.

## 작업

### 1. macOS 모든 데스크톱과 전체 화면

- [x] 1.1 `Cargo.toml`의 macOS 대상 `objc2`, `objc2-app-kit`을 `tauri-nspanel` 2.1로 바꾸고, `platform`에 `plugins`를 만들어 `lib.rs`에서 붙인다. `show_over_full_screen`이 작은 창을 패널로 바꾸고 결정에 적은 style mask, collection behavior, 레벨을 준다. 확인: `pnpm dev`로 다른 앱의 전체 화면으로 스와이프했을 때 작은 창이 보이는지 먼저 본다. 안 보이면 멈추고 "위험" 첫 항목대로 묻는다 (FR-WINDOW-009)
- [x] 1.2 macOS 수동 확인: Ctrl+→로 데스크톱을 옮겨도 같은 자리에 보이고, 메뉴 막대 메뉴와 알림은 작은 창 위에 뜨며, 다른 앱으로 Cmd+Tab 해도 작은 창이 위에 남는다. 패널로 바꾼 뒤에도 배경이 투명하고, 끌어 옮길 수 있고, Cmd+W로 숨고 독 클릭으로 다시 보이며, 독 아이콘이 보인다. `cargo tree -d`에 objc2 중복이 없다 (FR-WINDOW-009, FR-WINDOW-001, FR-WINDOW-006, FR-WINDOW-007, FR-WINDOW-008)

### 2. 옮길 자리 계산(core)

- [x] 2.1 `test-writer`가 delta와 위 `relocation` 시그니처만 보고 `placement.rs`의 `tests`에 테스트를 쓴다(중심이 작업 영역 안/밖, 메뉴 막대나 독 영역, 모니터 셋 이상, primary None·범위 밖, displays 빔, 배율 있는 주 모니터, 항상 작업 영역 안이라는 성질). 확인: `pnpm test:rust`에서 새 테스트가 컴파일 오류 없이 실패한다 (FR-WINDOW-006, FR-WINDOW-010)
- [x] 2.2 `placement::relocation`을 구현한다. 확인: `pnpm test:rust` 통과 (FR-WINDOW-006, FR-WINDOW-010)

### 3. 다시 불러낼 때 위치 확인

- [x] 3.1 `window/main.rs`에 `keep_visible`을 만든다. `target_rect`에서 창 사각형을 읽는 부분을 함수로 빼 함께 쓰고, 결과가 Some이면 `place`로 옮긴다. 확인: `pnpm lint:rust`, `pnpm test:rust` 통과 (FR-WINDOW-006)
- [x] 3.2 `window::show_current`가 작은 창을 보이기 전에 `keep_visible`을 부르게 한다. 확인: 작은 창을 보조 모니터에 두고 Cmd+W로 닫은 뒤 보조 모니터를 빼고 독 아이콘을 누르면 주 모니터 오른쪽 아래에 보인다. 왼쪽 위로 옮겨 닫고 다시 부르면 그 자리에 보인다. Windows는 앱을 다시 실행해 같은 동작을 본다 (FR-WINDOW-006) — 수동 확인 보류: 사용자가 나중에 확인하기로 함

### 4. 모니터 구성 변경 감시

- [x] 4.1 `tasks/display_watch.rs`에 `spawn`을 만들고 `tasks/mod.rs`에 등록한 뒤 `lib.rs`의 `setup`에서 띄운다. 작은 창이 없거나 숨어 있으면 확인을 건너뛴다. 확인: `pnpm lint:rust`, `pnpm test:rust` 통과 (FR-WINDOW-010)
- [x] 4.2 수동 확인(macOS, Windows): 작은 창이 있는 보조 모니터를 빼면 2초 안에 주 모니터 오른쪽 아래에 보인다. 주 모니터에 있을 때 보조 모니터를 빼면 그대로 있다. 해상도를 낮추거나 배치를 바꿔 창 중심이 화면 밖으로 나가면 옮겨지고, 옮긴 뒤에도 논리 크기 320×320이다 (FR-WINDOW-010, SC-WINDOW-002, FR-WINDOW-004) — 수동 확인 보류: 사용자가 나중에 확인하기로 함
- [x] 4.3 수동 확인: 감시가 도는 동안 작은 창의 3D가 끊기지 않는다 (SC-WINDOW-002)

### 5. Windows 항상 위 확인

- [x] 5.1 Windows 수동 확인: 다른 앱으로 Alt+Tab 하거나 브라우저를 F11 전체 화면으로 바꾸거나 동영상을 전체 화면으로 재생해도 작은 창이 위에 보인다 (FR-WINDOW-001) — 수동 확인 보류: 사용자가 나중에 확인하기로 함
