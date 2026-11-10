# Rust에서의 함수형

SKILL.md의 원칙을 Rust와 Tauri 앱에 적용하는 문서입니다.

## 경계

| 코드 | 성격 | 여기에 두는 것 |
| --- | --- | --- |
| 로직 모듈의 순수 함수와 도메인 타입 | core | 계산, 검증, 상태 전이. IO가 필요하면 trait으로 받음 |
| IO가 본업인 모듈(파일, DB, 네트워크) | shell | core가 정의한 trait을 구현 |
| `#[tauri::command]` | shell | 입력 받기 → core 호출 → 상태·파일 반영 → 반환 |
| `tauri::State<T>`의 `Mutex<T>` | 공유 상태 | 원본 값. 갱신 계산은 core 함수 |
| 백그라운드 작업, `emit` | shell | 채널로 값을 받아 core로 계산하고 결과를 프런트엔드에 알림 |
| 진입점(`run()`, `setup`), OS별 코드 | shell | 의존성 조립, 플러그인, 상태 등록, 창 이벤트 |

- 의존 방향은 shell(command, 백그라운드 작업, 진입점) → 로직 모듈 하나뿐입니다. 로직 모듈은 `tauri`를 import하지 않습니다.
- core는 여기에 더해 `tokio`, `std::fs`, `std::net`, `SystemTime::now()`도 쓰지 않습니다. 시각이 필요하면 인수로 받습니다.
- core는 동기 함수로만 씁니다. `async`와 `.await`는 shell에만 둡니다.
- 직렬화, 에러, 타입 생성처럼 코드를 생성할 뿐 동작이 없는 derive(`serde`, `thiserror` 등)는 core에서 써도 됩니다.

## 사용하는 라이브러리

std만으로 대부분의 원칙을 지킬 수 있습니다. `Option`, `Result`, `?`, 반복자, enum과 `match`가 기본 도구입니다. 아래 크레이트는 std로 채우기 번거로운 부분만 맡습니다.

| 크레이트 | 맡는 일 | 쓰는 곳 |
| --- | --- | --- |
| `serde` (`features = ["derive"]`) | 값을 JSON으로 직렬화 | command 반환 값과 에러. Tauri가 요구함 |
| `thiserror` | 에러 enum의 `Display`, `Error`, `From`을 derive | 모든 에러 enum |
| `tokio` (`features = ["sync"]`) | async 채널(`mpsc`, `watch`) | 백그라운드 작업에 값을 넘길 때. Tauri가 이미 의존하므로 새로 받는 크레이트는 없음 |
| `proptest` (dev-dependency) | 속성 기반 테스트 | core 함수의 성질 검증 |
| `itertools` | `chunk_by`, `tuple_windows`, `partition_map` 등 반복자 확장 | std 반복자로 길어질 때만 |

쓰지 않는 것:

- `anyhow`: 에러 종류를 지워서 TS 쪽이 `kind`로 분기할 수 없습니다. command와 core에서는 쓰지 않습니다.
- `frunk`, `higher` 같은 Haskell식 추상화(HKT, Monoid, HList): 관용적인 Rust가 아니고 읽는 사람의 부담만 늘립니다.
- `im`: 관리가 멈췄습니다. 영속 자료구조가 정말 필요하면 측정한 뒤 `imbl`을 검토합니다.
- mock 크레이트: 테스트는 trait을 직접 구현한 구조체로 합니다.

## 불변과 소유권

Rust 컴파일러는 "여러 곳에서 공유하거나, 한 곳에서 바꾸거나" 둘 중 하나만 허용합니다. 그래서 JS처럼 방어적으로 복사하지 않아도 숨은 공유 변경이 생기지 않습니다.

- **기본은 소유권을 받아 새 값을 반환하는 모양입니다.** `fn add_item(cart: Cart, item: String) -> Cart`는 안에서 `cart`를 고쳐도, 호출자가 이전 값을 더 이상 쓸 수 없습니다. 그래서 복사 비용 없이 순수 함수처럼 동작합니다.
- **호출자가 원본도 계속 써야 하면 `&T`를 받아 새 값을 반환합니다.** 복제 비용은 이 경우에만 냅니다.
- **실패할 수 있는 전이는 `&T`로 받습니다.** 소유권을 받은 채로 `Err`를 반환하면 호출자는 원래 값을 잃습니다.
- `&mut T`나 `&mut self`를 받는 함수는 "바꾼다"를 시그니처로 정직하게 말합니다. 공개 API에서는 위 두 모양을 먼저 고려합니다.
- 함수 안의 지역 가변성(`let mut acc`, 소유한 값의 `push`)은 바깥에 보이지 않으므로 순수성을 해치지 않습니다.
- 내부 가변성(`Cell`, `RefCell`)은 core에서 쓰지 않습니다. 숨은 출력이 됩니다.

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cart {
    pub items: Vec<String>,
}

// 소유권을 받아 고친 값을 반환한다. 복사가 없다
pub fn add_item(mut cart: Cart, item: String) -> Cart {
    cart.items.push(item);
    cart
}

// 원본을 남겨야 할 때만 &T를 받고 복제한다
pub fn with_item(cart: &Cart, item: &str) -> Cart {
    add_item(cart.clone(), item.to_owned())
}

// 파생 값은 저장하지 않고 계산한다
pub fn discount_percent(cart: &Cart) -> u32 {
    if cart.items.iter().any(|i| i == "book") { 5 } else { 0 }
}
```

## 반복자 = 고차 함수

| 연산 | Rust |
| --- | --- |
| map / filter / flatMap | `map`, `filter`, `flat_map` |
| filter + map | `filter_map` |
| fold | `fold(init, f)`, `sum`, `product` |
| all / any | `all`, `any` |
| 순서대로 짝짓기 | `zip`, `windows(n)` (슬라이스) |
| 결과 모으기 | `collect::<Vec<_>>()`, `collect::<Result<Vec<_>, _>>()` |

- 반복자는 지연 평가됩니다. `collect`나 `fold` 같은 소비 메서드를 부를 때 계산됩니다.
- for 루프와 `push`로 결과를 쌓는 코드는 대개 `map`/`filter`/`collect`로 바꿀 수 있습니다.

```rust
// 함수를 인수로
pub fn rank_by<T: Clone, S: Ord>(items: &[T], score: impl Fn(&T) -> S) -> Vec<T> {
    let mut sorted = items.to_vec(); // 지역 복사본을 정렬한다
    sorted.sort_by_key(|x| std::cmp::Reverse(score(x)));
    sorted
}

// 함수를 반환값으로 (부분 적용)
pub fn scoring_above<T: Clone>(score: impl Fn(&T) -> i64, threshold: i64) -> impl Fn(&[T]) -> Vec<T> {
    move |items| items.iter().filter(|x| score(x) > threshold).cloned().collect()
}

// 최근 값들이 계속 오르는가
pub fn is_rising(values: &[i64]) -> bool {
    values.len() > 1 && values.windows(2).all(|w| w[1] > w[0])
}
```

## 없음과 실패: `Option`, `Result`

- 없을 수 있으면 `Option<T>`, 실패 이유가 필요하면 `Result<T, E>`를 반환합니다.
- `unwrap()`, `expect()`, `panic!`, 인덱스 접근(`v[i]`)은 불가능한 상태에만 씁니다. 예상 가능한 실패에 쓰면 시그니처가 거짓말을 합니다. 진입점에서 앱 초기화가 실패한 경우처럼 실패하면 앱을 띄울 수 없는 곳은 예외입니다.
- `?`로 첫 실패에서 멈추는 파이프라인을 씁니다. 조합 메서드: `map`, `and_then`, `or_else`, `ok_or`, `ok_or_else`, `map_err`, `unwrap_or`, `unwrap_or_else`.
- `unwrap_or`, `ok_or`, `or`는 인수를 미리 계산합니다. 상수나 이미 있는 값이면 그대로 쓰고, 계산이나 할당(`format!`, `to_string()`, 함수 호출)이 필요한 대안은 `unwrap_or_else`, `ok_or_else`, `or_else`에 클로저로 넘겨 실패했을 때만 만들게 합니다.
- 실패 이유는 `String`이 아니라 `thiserror`로 만든 `enum`으로 닫습니다. trait의 에러 타입, command의 에러 타입도 같습니다.
- `#[error("...")]` 문구는 로그와 디버깅용입니다. 화면에 띄울 문구는 프런트엔드가 `kind`를 보고 고릅니다.
- 프런트엔드로 넘어갈 수 있는 에러 enum은 `#[serde(tag = "kind", rename_all = "camelCase")]`를 붙이고 variant를 unit이나 struct 모양으로 씁니다. 이유는 [Tauri command는 얇은 shell](#tauri-command는-얇은-shell)에 있습니다.

```rust
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PercentError {
    #[error("값이 비어 있음")]
    Empty,
    #[error("숫자가 아님: {raw}")]
    NotANumber { raw: String },
    #[error("범위 밖: {value}")]
    OutOfRange { value: i64 },
}

// newtype + 생성 함수: 검증을 통과한 값만 만들 수 있다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Percent(u8);

impl Percent {
    pub const FULL: Percent = Percent(100);

    pub fn parse(raw: &str) -> Result<Percent, PercentError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(PercentError::Empty);
        }
        let n: i64 = trimmed.parse().map_err(|_| PercentError::NotANumber {
            raw: trimmed.to_owned(),
        })?;
        u8::try_from(n)
            .ok()
            .filter(|p| *p <= 100)
            .map(Percent)
            .ok_or(PercentError::OutOfRange { value: n })
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

// all-or-nothing: Result의 반복자를 Result<Vec<_>>로 모으면 첫 실패에서 멈춘다
pub fn parse_all(raws: &[&str]) -> Result<Vec<Percent>, PercentError> {
    raws.iter().map(|r| Percent::parse(r)).collect()
}

// best-effort: 성공은 모으고 실패 이유도 버리지 않는다
pub fn parse_some(raws: &[&str]) -> (Vec<Percent>, Vec<PercentError>) {
    raws.iter()
        .map(|r| Percent::parse(r))
        .fold((Vec::new(), Vec::new()), |(mut oks, mut errs), r| {
            match r {
                Ok(p) => oks.push(p),
                Err(e) => errs.push(e),
            }
            (oks, errs)
        })
}

// 대안을 값으로 붙인다. 상수 대안은 바로 넘긴다
pub fn percent_or_full(raw: &str) -> Percent {
    Percent::parse(raw).unwrap_or(Percent::FULL)
}

// 계산이 필요한 대안은 클로저로 넘겨 실패했을 때만 계산한다
pub fn percent_or_else(raw: &str, fallback: impl FnOnce() -> Percent) -> Percent {
    Percent::parse(raw).unwrap_or_else(|_| fallback())
}
```

- newtype의 필드를 비공개로 두면(`struct Percent(u8)`) 모듈 밖에서는 생성 함수를 거쳐야만 값을 만들 수 있습니다. 그러면 검증 함수가 유일한 입구가 됩니다.

## 요구사항을 타입으로: enum과 match

상태마다 필요한 데이터를 variant에 넣으면 `is_running: bool`과 `started_at: Option<u64>`가 어긋나는 조합을 만들 수 없습니다. 상태 전이는 "현재 상태 + 이벤트 → 새 상태"인 순수 함수로 씁니다.

```rust
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Job {
    Queued,
    Running { started_at_ms: u64 },
    Done { took_ms: u64 },
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEvent {
    Start { now_ms: u64 },
    Finish { now_ms: u64 },
    Fail { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TransitionError {
    #[error("지금 상태에서 받을 수 없는 이벤트")]
    NotAllowed,
}

// 실패할 수 있는 전이라 &Job으로 받는다. 실패해도 호출자의 원래 상태는 남는다
pub fn next(job: &Job, event: &JobEvent) -> Result<Job, TransitionError> {
    match job {
        Job::Queued => match event {
            JobEvent::Start { now_ms } => Ok(Job::Running { started_at_ms: *now_ms }),
            JobEvent::Finish { .. } | JobEvent::Fail { .. } => Err(TransitionError::NotAllowed),
        },
        Job::Running { started_at_ms } => match event {
            JobEvent::Start { .. } => Err(TransitionError::NotAllowed),
            JobEvent::Finish { now_ms } => Ok(Job::Done {
                took_ms: now_ms.saturating_sub(*started_at_ms),
            }),
            JobEvent::Fail { reason } => Ok(Job::Failed { reason: reason.clone() }),
        },
        Job::Done { .. } | Job::Failed { .. } => Err(TransitionError::NotAllowed),
    } // 상태나 이벤트를 추가하면 여기서 컴파일 에러가 난다
}
```

- 도메인 enum의 `match`에 `_ =>`나 `(a, b) =>` 같은 catch-all을 쓰지 않습니다. 위처럼 바깥과 안쪽 `match`가 모두 variant를 빠짐없이 나열하면, 새 variant가 생겼을 때 컴파일러가 처리할 곳을 알려 줍니다.
- Rust는 상태를 값으로 반환하고, 화면 문구나 모션 같은 표현은 프런트엔드가 고릅니다. core에서 UI 문자열을 만들지 않습니다.
- `#[serde(tag = "kind")]`를 붙인 enum은 TS에서 판별 유니온(`{ kind: 'running'; startedAtMs: number } | ...`)으로 받습니다. enum의 `rename_all`은 variant 이름(`kind` 값)만 바꿉니다. variant 안의 필드 이름까지 camelCase로 바꾸려면 `rename_all_fields`를 함께 붙입니다.

## 사이드 이펙트와 의존성 주입

- 계산이 외부 기능을 필요로 하면 trait(시그니처 묶음)이나 클로저로 받습니다. 테스트에서는 고정 값을 돌려주는 구현을 넘깁니다.
- trait은 동기로 둡니다. async IO는 shell에서 끝내고, 그 결과 값을 core에 넘깁니다.

```rust
pub trait Clock {
    fn now_ms(&self) -> u64;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    #[error("저장소를 읽을 수 없음: {message}")]
    Unavailable { message: String },
    #[error("저장된 값이 깨짐: {raw}")]
    Corrupted { raw: String },
}

pub trait Store {
    fn load(&self, key: &str) -> Result<Option<String>, StoreError>;
}

// 순수: 시각을 인수로
pub fn is_expired(expires_at_ms: u64, now_ms: u64) -> bool {
    now_ms >= expires_at_ms
}

// 순수: 저장된 문자열을 파싱한다. 깨진 값을 기본값으로 덮지 않고 실패 이유로 돌려준다
pub fn parse_expiry(raw: &str) -> Result<u64, StoreError> {
    raw.parse().map_err(|_| StoreError::Corrupted { raw: raw.to_owned() })
}

// 조립: 의존성을 받아 IO와 계산을 잇는다
pub fn cache_expired(store: &impl Store, clock: &impl Clock, key: &str) -> Result<bool, StoreError> {
    match store.load(key)? {
        None => Ok(true), // 캐시가 없으면 만료로 본다 (규칙이므로 코드에 드러낸다)
        Some(raw) => Ok(is_expired(parse_expiry(&raw)?, clock.now_ms())),
    }
}

// 재시도도 함수 조합
pub fn retry<T, E>(times: usize, mut action: impl FnMut() -> Result<T, E>) -> Result<T, E> {
    let mut result = action();
    for _ in 0..times {
        if result.is_ok() {
            break;
        }
        result = action();
    }
    result
}
```

## Tauri command는 얇은 shell

```rust
use serde::Serialize;
use std::sync::Mutex;
use tauri::Manager;

// 공유 상태
#[derive(Default)]
pub struct AppState {
    pub cart: Mutex<Cart>,
}

// command가 돌려주는 공통 에러
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CommandError {
    #[error("상태 잠금 실패")]
    StateLock,
    #[error("입력이 잘못됨: {source}")]
    InvalidPercent {
        #[from]
        source: PercentError,
    },
    #[error("입출력 실패: {message}")]
    Io { message: String },
}

#[tauri::command]
fn add_to_cart(state: tauri::State<'_, AppState>, item: String) -> Result<u32, CommandError> {
    let mut cart = state.cart.lock().map_err(|_| CommandError::StateLock)?;
    // 잠금 안에서는 순수 계산만. 값을 꺼내 소유권 버전 함수에 넘기고 결과를 되돌려 놓는다
    *cart = add_item(std::mem::take(&mut *cart), item);
    Ok(discount_percent(&cart))
}

#[tauri::command]
fn check_percent(raw: String) -> Result<u8, CommandError> {
    Ok(Percent::parse(&raw)?.value()) // PercentError → CommandError는 #[from]이 바꾼다
}

// 로직 모듈의 순수 함수: 내보낼 내용을 만든다
pub fn render_cart(cart: &Cart) -> String {
    cart.items.join("\n")
}

#[tauri::command]
async fn export_cart(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), CommandError> {
    let io = |e: &dyn std::fmt::Display| CommandError::Io { message: e.to_string() };
    // 1. 잠금은 이 문장이 끝날 때 풀린다. 필요한 값만 복제해 꺼낸다
    let snapshot = state.cart.lock().map_err(|_| CommandError::StateLock)?.clone();
    // 2. 순수 계산
    let text = render_cart(&snapshot);
    // 3. IO는 잠금 밖에서, 블로킹 IO는 spawn_blocking으로
    let dir = app.path().app_data_dir().map_err(|e| io(&e))?;
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("cart.txt"), text)
    })
    .await
    .map_err(|e| io(&e))?
    .map_err(|e| io(&e))
}
```

- command는 "입력 → core → 반영 → 반환"만 합니다. 조건 분기가 생기면 core 함수로 옮깁니다.
- `Mutex`를 잡은 동안에는 순수 계산만 하고, 파일과 네트워크 IO는 잠금을 풀고 합니다. 필요하면 값을 복제해 잠금 밖으로 가져갑니다.
- `std::mem::take`로 값을 꺼내는 방식은 갱신 함수가 실패하지 않을 때만 씁니다. 실패할 수 있는 전이는 `&T`를 받는 함수로 계산하고, 성공했을 때만 대입합니다(`*job = next(&job, &event)?;`).
- **command의 에러 타입은 `Serialize`여야 합니다.** Tauri가 에러 값을 JSON으로 바꿔 `invoke`의 reject 값으로 넘깁니다. `#[serde(tag = "kind")]`를 붙이면 TS에서 `{ kind: 'stateLock' } | { kind: 'invalidPercent'; source: PercentError } | { kind: 'io'; message: string }`로 받습니다. TS의 IO 경계 모듈에서 `ResultAsync.fromPromise`로 감쌉니다([typescript.md](typescript.md)). reject 값은 이 유니온일 수도 있고, 없는 명령이나 인자 불일치처럼 Tauri가 보낸 문자열일 수도 있으므로 둘을 구분합니다.
- **`tag`를 붙인 enum에는 `NotANumber(String)`처럼 원시 타입 하나를 감싼 variant를 쓰지 않습니다.** 컴파일은 되지만 직렬화할 때 런타임 에러가 납니다. unit(`StateLock`)이나 struct(`Io { message }`) 모양으로 씁니다.
- `State`나 `&str`처럼 빌린 인수를 받는 async command는 `Result`를 반환해야 컴파일됩니다.
- 새 command는 `generate_handler!`에, 상태는 `.manage(AppState::default())`에 등록합니다.

## 비동기와 백그라운드 작업

- **`std::sync::Mutex`의 guard를 쥔 채 `.await`하지 않습니다.** async command는 `Send` future여야 하므로 컴파일러가 막습니다. 블록이나 문장 하나로 잠금 범위를 끝낸 뒤 `.await`합니다. guard를 꼭 `.await` 너머로 들고 가야 할 때만 `tokio::sync::Mutex`를 씁니다.
- 백그라운드 작업은 `tauri::async_runtime::spawn`으로 띄우고, 입력은 채널로 값을 받습니다. 송신자(`Sender`)를 모두 drop하면 `recv()`가 `None`을 반환해 루프가 끝나므로 중지 신호를 따로 두지 않아도 됩니다.
- 프런트엔드에 알리는 `emit`은 shell에서 합니다. 보내는 값은 core가 계산한 결과입니다.
- 작업을 시작하는 함수는 `JoinHandle`을 반환해 호출자가 기다리거나 중지할 수 있게 합니다.

```rust
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;

// 이벤트 이름은 상수로 두고 프런트엔드와 같은 값을 쓴다
pub const CART_CHANGED: &str = "cart-changed";

pub fn spawn_cart_loop(
    app: tauri::AppHandle,
    mut rx: mpsc::Receiver<String>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        while let Some(item) = rx.recv().await {
            let count = {
                let state = app.state::<AppState>();
                // 다른 스레드가 잠금을 쥔 채 panic하면 잠금이 오염된다. 그때는 루프를 끝낸다
                let Ok(mut cart) = state.cart.lock() else { break };
                *cart = add_item(std::mem::take(&mut *cart), item);
                cart.items.len()
            }; // guard는 여기서 풀린다. 이 뒤에서 IO와 await를 한다
            if app.emit(CART_CHANGED, count).is_err() {
                break;
            }
        }
    })
}
```

- 이 루프는 `run()`의 `setup`에서 `mpsc::channel`을 만든 뒤 시작합니다. 송신자는 입력을 만드는 쪽(다른 작업, 서버 핸들러 등)에 넘깁니다.

## 동시성과 자원

- 공유 상태는 `Mutex<T>`/`RwLock<T>`로 감싸고, 갱신은 "현재 값 → 새 값" core 함수로 합니다. 잠금 범위를 최소로 유지합니다.
- 작업 사이에 데이터를 넘길 때는 공유 상태보다 채널로 값을 보내는 쪽을 먼저 고려합니다. 생산자와 소비자가 자연스럽게 나뉩니다. async 쪽은 `tokio::sync::mpsc`를, 스레드끼리는 `std::sync::mpsc`를 씁니다.
- 자원 해제는 `Drop`(RAII)에 맡깁니다. 파일, 잠금 guard, 연결은 범위를 벗어나면 해제되므로 수동 close보다 범위를 좁히는 쪽을 택합니다.

## 테스트

- core 함수는 `#[cfg(test)] mod tests`에서 Tauri 없이 바로 테스트합니다.
- trait 의존성은 테스트용 구조체로 구현해 고정 값을 돌려줍니다.
- 예시 몇 개로 확인하기 어려운 성질(상한을 넘지 않는다, 순서와 무관하다, 되돌리면 원래 값이다)은 `proptest`로 검증합니다.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    struct FixedClock(u64);
    impl Clock for FixedClock {
        fn now_ms(&self) -> u64 {
            self.0
        }
    }

    struct MemStore(Option<String>);
    impl Store for MemStore {
        fn load(&self, _key: &str) -> Result<Option<String>, StoreError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn expired_when_now_passes_deadline() {
        let store = MemStore(Some("100".into()));
        assert_eq!(cache_expired(&store, &FixedClock(150), "k"), Ok(true));
        assert_eq!(cache_expired(&store, &FixedClock(50), "k"), Ok(false));
    }

    #[test]
    fn corrupted_expiry_is_an_error_not_expired() {
        let store = MemStore(Some("soon".into()));
        assert_eq!(
            cache_expired(&store, &FixedClock(0), "k"),
            Err(StoreError::Corrupted { raw: "soon".into() })
        );
    }

    #[test]
    fn parse_all_stops_at_first_error() {
        assert_eq!(
            parse_all(&["80", "x", ""]),
            Err(PercentError::NotANumber { raw: "x".into() })
        );
    }

    #[test]
    fn finished_job_rejects_events() {
        let done = Job::Done { took_ms: 3 };
        assert_eq!(next(&done, &JobEvent::Start { now_ms: 0 }), Err(TransitionError::NotAllowed));
    }

    proptest! {
        #[test]
        fn adding_items_keeps_all_of_them(items in prop::collection::vec(".*", 0..20)) {
            let cart = items.iter().cloned().fold(Cart::default(), add_item);
            prop_assert_eq!(cart.items, items);
        }

        #[test]
        fn valid_percents_round_trip(p in 0u8..=100) {
            prop_assert_eq!(Percent::parse(&p.to_string()).map(Percent::value), Ok(p));
        }
    }
}
```

- 반복자 관련 clippy 제안(`needless_range_loop`, `manual_filter_map` 등)은 대개 함수형 방향과 같으니 따릅니다.

## 체크리스트

1. 계산 함수가 소유권을 받아 새 값을 반환하거나 `&T`를 받아 새 값을 반환하는가? 원본이 필요 없는데 복제하고 있지 않은가?
2. 실패할 수 있는 전이가 소유권을 받아서, 실패하면 원래 값을 잃는가?
3. 예상 가능한 실패에 `unwrap`, `expect`, `panic!`, 인덱스 접근을 쓰는가?
4. 실패 이유가 `String`이 아니라 `thiserror` enum으로 닫혀 있는가? trait과 command의 에러도 같은가?
5. 프런트엔드로 가는 enum에 `#[serde(tag = "kind")]`가 있고, variant가 unit이나 struct 모양인가?
6. 도메인 enum의 `match`에 `_ =>` 같은 catch-all이 있는가?
7. 같은 원시 타입 파라미터 여러 개를 newtype으로 구분하는가?
8. `#[tauri::command]` 안에 계산 규칙이 있는가?
9. `Mutex`를 잡은 채 IO를 하거나 `.await`하는가?
10. core가 `tauri`, `tokio`, `std::fs`, `SystemTime::now()`, `async`를 쓰는가? UI 문구를 만드는가?
