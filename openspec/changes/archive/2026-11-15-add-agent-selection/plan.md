# Plan

spec-hash: 918a3447d26c1066

## 설계

### 층 배치

| 층 | 파일 | 하는 일 |
| --- | --- | --- |
| core(Rust) | `src-tauri/src/model/domain/model.rs` | 모델 선택지, 모델 목록, 선택 값 타입 |
| core(Rust) | `src-tauri/src/model/domain/parse.rs` | Claude 초기화 응답, `codex debug models` 출력, Codex `config.toml`의 `model`을 해석 |
| core(Rust) | `src-tauri/src/model/domain/catalog.rs` | 조회 결과와 고정 목록으로 모델 목록과 기본 모델을 정함, 선택 값 저장 형식 |
| IO(Rust) | `src-tauri/src/model/host.rs`, `model/query.rs` | CLI를 실행하는 `ModelHost` trait, 시간 상한을 걸고 조회해 core에 넘기는 흐름 |
| core(Rust) | `src-tauri/src/placement.rs` | 모니터 고르기, 작업 영역 오른쪽 아래 위치와 크기 계산, 다시 보일 창 고르기. 모듈 전체가 순수해서 `domain/`을 두지 않음 |
| IO(Rust) | `src-tauri/src/platform/macos.rs`, `windows.rs`, `process.rs` | 환경별 `ModelHost` 구현(macOS 로그인 셸 `PATH`, Windows, WSL `wsl.exe`), 표준 입력에 한 줄 쓰고 응답 줄을 기다리는 프로세스 도우미, 창 닫기 동작 |
| 공유 상태(Rust) | `src-tauri/src/state.rs` | 고른 값 보관, 창 전환을 한 번에 하나만 하는 잠금 |
| 창(Rust) | `src-tauri/src/window/main.rs` | 작은 창 만들기(숨긴 채 만들고 위치·크기를 정한 뒤 보임) |
| 창(Rust) | `src-tauri/src/window/mod.rs` | 지금 쓰는 창을 앞으로 가져오기(single-instance, macOS 독 클릭이 같이 씀) |
| 명령 | `src-tauri/src/commands/model.rs`, `commands/onboarding.rs`, `lib.rs` | 모델 목록 조회, 저장한 선택 읽기, 다음(저장 → 작은 창 열기 → 시작 설정 창 닫기), 명령 등록 |
| 앱 설정 | `src-tauri/tauri.conf.json`, `Cargo.toml`, `capabilities/onboarding.json`, `capabilities/main.json` | 처음 창을 `onboarding`으로, `macOSPrivateApi`와 `macos-private-api` feature, 창별 권한 |
| IO(TS) | `src/renderer/src/api/index.ts` | 새 명령, 창 끌기 시작 |
| core(TS) | `src/renderer/src/features/onboarding/domain/detectionView.ts` | 기존 `features/agent-detection/detectionView.ts`를 옮김(FR-AGENT-008) |
| core(TS) | `src/renderer/src/features/onboarding/domain/installView.ts` | 기존 `hookEventsView.ts`에서 설치 결과 줄 변환만 옮김(FR-HOOK-015) |
| 명령 에러(Rust) | `src-tauri/src/error.rs` | `OnboardingError` |
| core(TS) | `src/renderer/src/features/onboarding/domain/steps.ts` | 세 단계의 진행 상태 |
| core(TS) | `src/renderer/src/features/onboarding/domain/selection.ts` | 에이전트 선택지, 조회할 항목 고르기, 선택 상태 전이(미리 고르기, 다시 감지 뒤 유지), 다음 버튼 가능 여부 |
| React | `src/renderer/src/features/onboarding/*.tsx` | 감지 단계, 훅 설치 단계, 선택 단계, 이 셋을 묶는 `Onboarding` |
| React | `src/renderer/src/features/window-drag/WindowDragArea.tsx` | 누르면 창 끌기를 시작하는 영역 |
| 3D | `src/renderer/src/scene/TestScene.tsx` | 투명 배경 위에서 계속 도는 시험용 도형. 기존 `DemoScene.tsx`를 바꿈 |
| 화면 조립 | `src/renderer/src/windows/onboarding/`, `windows/main/`, `onboarding.html`, `index.html`, `vite.config.ts` | 시작 설정 창 진입점 추가, `main`을 작은 창으로 바꿈, 두 HTML을 빌드 입력에 등록 |
| 정리 | `features/versions/`, `features/agent-detection/`, `features/hook-events/` | 지움. 남길 core와 테스트는 `features/onboarding/domain/`으로 옮김 |

### 타입과 경계

ts-rs로 내보내는 새 타입(`model::domain::model`):

```rust
/// 모델 선택지 하나. `value`는 에이전트에 넘기는 값(`opus`, `gpt-5.6-sol` 등)이다.
pub struct ModelOption {
    pub value: String,
    /// 화면에 보이는 이름(`Opus 5.5`, `GPT-5.6-Sol`)
    pub label: String,
    /// 별칭(`default`, `opus` 등)이 지금 가리키는 실제 모델 ID(`claude-opus-5-5`). 별칭이 아니거나 모르면 None.
    /// CLI가 준 설명문은 넣지 않는다(FR-START-004)
    #[ts(optional)] pub detail: Option<String>,
}

/// 모델 목록 조회 결과.
pub struct ModelList {
    pub options: Vec<ModelOption>,
    /// 미리 고를 모델의 `value`. 항상 `options` 안에 있다
    pub default_value: String,
    /// `queried`: CLI에서 받음, `fallback`: 실패하거나 시간을 넘겨 고정 목록을 씀
    pub source: ModelSource,
}

/// 다음을 눌렀을 때 고른 값. 저장 파일과 명령 인자에 그대로 쓴다.
pub struct Selection {
    pub agent: AgentKind,
    pub environment: Environment,
    pub model: String,
}
```

- `AgentKind`, `Environment`에 `Deserialize`를 더합니다. 명령 인자와 저장 파일을 읽을 때 씁니다.
- 새 명령
  - `agent_models(agent, environment) -> ModelList`: 실패하지 않습니다. 실패하면 `source: fallback`입니다.
  - `saved_selection() -> Option<Selection>`: 앱 데이터 폴더의 `selection.json`을 읽습니다. 지금 실행에서 다음을 눌렀다면 그 값을 줍니다. 반환값이 없으면 `null`로 오므로 `api/`가 `undefined`로 바꿉니다.
  - `finish_onboarding(selection) -> Result<(), OnboardingError>`: 다음 버튼입니다. 처리 순서는 이렇습니다.
    1. 선택을 상태에 넣고 파일에 씁니다. 파일 쓰기가 실패해도 계속합니다.
    2. 작은 창이 없으면 만들고, 이미 있으면 앞으로 가져옵니다.
    3. 시작 설정 창을 닫습니다(`destroy`).
- `OnboardingError { kind: "windowFailed", message }`: 작은 창을 만들지 못한 경우입니다. 이때 시작 설정 창은 닫지 않습니다.
- 이벤트: 새로 추가하지 않습니다. `HOOK_INSTALLATION`과 `HOOK_EVENT`는 그대로 모든 창에 갑니다.
- 권한
  - `capabilities/onboarding.json`: `core:default`
  - `capabilities/main.json`: `core:default`, `core:window:allow-start-dragging`
  - 앱 명령은 지금처럼 모든 창이 부를 수 있습니다.
- `tauri.conf.json`
  - `app.windows`에는 `onboarding` 창만 둡니다. url은 `onboarding.html`, 크기는 720×560, 가운데에 엽니다.
  - `main` 창은 Rust가 만듭니다(url `index.html`).
  - `app.macOSPrivateApi: true`를 켭니다.

### 시그니처

Rust core(`model/domain/parse.rs`):

```rust
/// Claude Code stream-json 출력 한 줄이 초기화 응답이면 그 안의 모델 목록을 꺼낸다.
/// 응답 모양: `{"type":"control_response","response":{"subtype":"success","request_id":<id>,"response":{"models":[...]}}}`.
/// `request_id`가 다르거나 subtype이 success가 아니거나 초기화 응답이 아니면 None.
/// 모델 항목은 value→value, displayName→label을 읽고 둘 중 하나라도 없는 항목은 건너뛴다.
/// resolvedModel이 있고 value와 다르면 detail에 넣는다(별칭). description은 읽지 않는다. 응답 안의 순서를 그대로 둔다.
pub fn claude_models(line: &str, request_id: &str) -> Option<Vec<ModelOption>>;

/// `codex debug models` 출력에서 visibility가 "list"인 모델만 priority 오름차순으로 꺼낸다.
/// JSON이 아니거나 `models` 배열이 없으면 None. value는 slug, label은 display_name(없으면 slug), detail은 None.
pub fn codex_models(output: &str) -> Option<Vec<ModelOption>>;

/// Codex config.toml의 최상위 `model` 문자열. 파일이 TOML이 아니거나 값이 없거나 빈 문자열이면 None.
pub fn codex_config_model(config: &str) -> Option<String>;
```

Rust core(`model/domain/catalog.rs`):

```rust
/// 에이전트별 고정 목록. Claude: default, opus, sonnet, fable, haiku. Codex: gpt-5.6-sol, gpt-5.6-terra, gpt-5.6-luna.
/// 별칭이 가리키는 실제 모델을 알 수 없으므로 detail은 모두 None이다.
pub fn fallback_models(agent: AgentKind) -> Vec<ModelOption>;

/// 조회 결과로 목록과 기본 모델을 정한다.
/// - queried가 None이거나 빈 목록이면 고정 목록을 쓰고 source는 Fallback.
/// - Claude: 기본은 "default". 목록에 없으면 첫 항목.
/// - Codex: 기본은 config_model. 목록에 없으면 label=value인 선택지로 맨 앞에 더한다.
///   config_model이 None이면 첫 항목.
pub fn model_list(agent: AgentKind, queried: Option<Vec<ModelOption>>, config_model: Option<String>) -> ModelList;

/// selection.json 내용을 읽는다. JSON이 아니거나 필드가 모자라면 None.
pub fn parse_selection(contents: &str) -> Option<Selection>;
/// selection.json에 쓸 내용.
pub fn selection_json(selection: &Selection) -> String;
```

Rust IO(`model/host.rs`, `model/query.rs`):

```rust
/// 모델 목록을 묻는 CLI 실행. platform/이 OS마다 구현하고, 테스트는 가짜 구현을 쓴다.
pub trait ModelHost: Sync {
    /// Claude를 stream-json 모드로 띄워 초기화 요청 한 줄을 쓰고, `accept`가 Some을 돌려주는 줄이 나올 때까지 읽는다.
    /// 훅과 MCP를 끄고 세션을 저장하지 않는다. 프로세스는 응답을 받거나 future가 버려지면 죽인다.
    fn claude_init<T: Send>(&self, environment: &Environment, path: &str, request: &str,
        accept: impl Fn(&str) -> Option<T> + Send + Sync) -> impl Future<Output = Option<T>> + Send;
    /// `codex debug models`의 stdout. 실패하면 None.
    fn codex_models(&self, environment: &Environment, path: &str) -> impl Future<Output = Option<String>> + Send;
    /// 그 환경의 `~/.codex/config.toml` 내용. 없거나 읽지 못하면 None.
    fn codex_config(&self, environment: &Environment) -> impl Future<Output = Option<String>> + Send;
}

/// 모델 목록 조회 한 번의 상한. SC-START-001
pub const MODEL_QUERY_LIMIT: Duration = Duration::from_secs(5);

/// 한 항목의 모델 목록. `limit` 안에 조회가 끝나지 않으면 고정 목록이다.
pub async fn query_models(host: &impl ModelHost, agent: AgentKind, environment: &Environment, path: &str, limit: Duration) -> ModelList;
```

Rust core(`placement.rs`). 좌표는 모두 물리 픽셀이고, 배율은 모니터마다 다릅니다.

```rust
pub struct Rect { pub x: i32, pub y: i32, pub width: u32, pub height: u32 }
pub struct Display { pub bounds: Rect, pub work_area: Rect, pub scale: f64 }

/// 창 사각형의 중심이 들어 있는 모니터의 번호. 없으면 primary(주 모니터 번호). primary도 None이면 0. displays가 비면 None.
pub fn pick_display(displays: &[Display], window: Rect, primary: Option<usize>) -> Option<usize>;

/// 논리 크기 `size`(320×320)의 창을 그 모니터 작업 영역의 오른쪽 아래에, 논리 여백 `margin`(16)만큼 띄워 둘 물리 사각형.
/// 크기와 여백은 그 모니터 배율을 곱하고 반올림한다. 가로와 세로는 따로 정한다.
/// 한 축의 창 크기가 작업 영역보다 크면 그 축만 작업 영역 크기로 줄이고, 여백은 그 축에 남는 만큼만(0~margin×배율) 둔다.
/// 결과는 항상 작업 영역 안에 있다.
pub fn corner_rect(display: &Display, size: (f64, f64), margin: f64) -> Rect;

/// 앱을 다시 불러낼 때 보일 창. 작은 창이 있으면 "main", 없으면 "onboarding".
pub fn window_to_show(main_exists: bool) -> &'static str;
```

TS core(`features/onboarding/domain/`):

```ts
// steps.ts
export type StepStatus = 'pending' | 'running' | 'done';
/**
 * 감지·훅 설치·선택 단계의 상태. detecting이면 감지 running이고 나머지는 pending(다시 감지 중 포함).
 * 감지가 끝났고 installing이면 설치 running, 선택 pending. 둘 다 끝나면 감지·설치 done, 선택 running.
 */
export function stepStatuses(input: { detecting: boolean; hasDetection: boolean; installing: boolean; hasInstallation: boolean }):
  { detect: StepStatus; install: StepStatus; select: StepStatus };

// selection.ts
/** 에이전트 선택지 하나. 감지 결과에서 설치됨인 항목만, entries 순서대로. */
export type AgentChoice = { key: string; agent: AgentKind; environment: Environment; label: string };
export function agentChoices(detection: Detection): readonly AgentChoice[];
/** 감지가 끝났을 때 바로 조회할 항목(WSL이 아닌 것 + 이미 골라진 WSL 항목). FR-START-007 */
export function choicesToQuery(choices: readonly AgentChoice[], selectedKey: string | undefined): readonly AgentChoice[];

/** 항목별 모델 목록 상태. 없으면 아직 조회하지 않음. */
export type ModelsState = { readonly [choiceKey: string]: { status: 'loading' } | { status: 'ready'; list: ModelList } };
export type SelectionState = { choices: readonly AgentChoice[]; selectedKey?: string; model?: string; models: ModelsState; saved?: Selection };
export type SelectionEvent =
  | { type: 'detected'; detection: Detection }   // 처음 감지와 다시 감지 모두
  | { type: 'savedLoaded'; saved: Selection | undefined }
  | { type: 'agentPicked'; key: string }
  | { type: 'modelsLoading'; key: string }
  | { type: 'modelsLoaded'; key: string; list: ModelList }
  | { type: 'modelPicked'; value: string };
/**
 * 선택 상태 전이.
 * - savedLoaded는 항상 첫 detected보다 먼저 온다(앱이 저장값을 먼저 읽는다). 역순은 다루지 않는다.
 * - detected: 선택지를 새로 만들고, 골라진 항목이 여전히 있으면 유지, 없으면 FR-START-003대로(하나면 그것, saved가 있으면 그것) 고른다.
 * - modelsLoaded: 그 항목이 골라져 있고 고른 모델이 새 목록에 없으면 saved 모델(목록에 있을 때) → default_value 순으로 고른다.
 * - agentPicked: 다른 항목이면 모델을 비우고, 목록이 ready면 default_value를 고른다.
 */
export function selectionReducer(state: SelectionState, event: SelectionEvent): SelectionState;
/** 다음 버튼을 누를 수 있는가. 감지 중이 아니고 에이전트와 모델이 모두 골라졌을 때만 true. */
export function canProceed(state: SelectionState, detecting: boolean): boolean;
/** 다음을 누를 때 넘길 값. 고르지 않았으면 undefined. */
export function toSelection(state: SelectionState): Selection | undefined;
```

옮기는 core는 시그니처를 바꾸지 않습니다.
- `detectionView.ts`: `toRows`, `panelReducer`, `canRedetect`, `failureLabel`
- `installView.ts`: `toInstallRows`, `installFailureLabel`

### 결정

- **Claude 모델 목록은 CLI에 직접 초기화 요청을 보내 받습니다.** Agent SDK(Node)를 쓰지 않습니다.
  - 요청: `claude -p --input-format stream-json --output-format stream-json --verbose --settings '{"disableAllHooks":true}' --strict-mcp-config --no-session-persistence`를 띄우고, `{"type":"control_request","request_id":…,"request":{"subtype":"initialize"}}` 한 줄을 씁니다.
  - 이유: 이 Mac에서 0.8~0.9초에 14개 모델을 받았습니다. 앱이 Node를 함께 배포하지 않아도 됩니다.
  - 작업 폴더: 사용자 홈입니다.
- **Codex 모델 목록은 `codex debug models`로 받습니다.** 이 Mac에서 0.8초 걸렸습니다. `config.toml`은 훅 설치와 같은 경로 규칙으로 읽습니다. WSL은 `\\wsl$`로 읽습니다.
- **조회 명령은 감지 결과의 실행 파일 경로로 실행합니다.**
  - macOS: 감지에 쓴 로그인 셸 `PATH`를 그대로 넘깁니다. npm으로 설치한 `claude`가 `node`를 찾아야 하기 때문입니다.
  - WSL: `wsl.exe -d <배포판> -- sh -lc`로 실행합니다. 감지 프로브와 같은 방식입니다.
- **조회 시점은 renderer가 정합니다.**
  - Rust는 부를 때마다 새로 조회합니다(캐시 없음).
  - 감지가 끝나면(다시 감지 포함) `choicesToQuery`로 고른 항목만 동시에 부릅니다. 고른 항목이 WSL이면 그 항목도 함께 부릅니다.
  - 버린 방안: 시점을 Rust에 두면 renderer의 선택 상태를 Rust가 알아야 해서 경계가 늘어납니다.
- **선택 저장 위치는 Tauri `app_data_dir()`의 `selection.json`입니다.** macOS는 `~/Library/Application Support/com.namasteworld.app/`이고, Windows는 `%APPDATA%\com.namasteworld.app\`입니다. 훅 파일을 두는 `~/.namaste-world`는 에이전트가 읽는 파일만 두는 곳이라 섞지 않습니다.
- **작은 창은 Rust에서 숨긴 채 만듭니다.** 위치와 크기를 정한 뒤 보입니다.
  - 순서: 고른 모니터의 작업 영역 안 임시 위치로 옮김 → 그 모니터 배율로 계산한 `corner_rect`를 물리 단위로 지정 → 보이기.
  - 이유: 배율이 다른 모니터에 바로 만들면, Windows에서 만든 직후 배율이 바뀌면서 크기가 틀어질 수 있습니다.
  - 버린 방안: 정적 창(`tauri.conf.json`)은 앱을 켤 때 함께 열려 FR-START-001을 어깁니다.
- **창 전환은 잠금 하나로 직렬화합니다.** 다음을 연달아 눌러도 두 번째 호출은 작은 창이 이미 있는 것을 보고 새로 만들지 않습니다(FR-WINDOW-001).
- **창 끌기는 `pointerdown`에서 `startDragging()`을 부릅니다.** `data-tauri-drag-region`은 자식 요소(Canvas)를 누를 때 동작하지 않을 수 있습니다. 시험용 3D에는 `OrbitControls`를 두지 않아 끌기와 충돌하지 않게 합니다.
- **투명 배경**
  - 창 설정: `transparent: true`, `decorations: false`, `alwaysOnTop: true`, `shadow: false`, `resizable: false`, `skipTaskbar: false`
  - `html`, `body`, Canvas 배경을 투명하게 합니다. 시작 설정 창 CSS와 작은 창 CSS는 파일을 나눕니다.
- **닫기 동작**
  - macOS: 지금처럼 모든 창의 닫기 요청을 숨기기로 바꿉니다.
  - 전환할 때만 `destroy()`로 시작 설정 창을 실제로 없앱니다.
  - 독 클릭과 다시 실행은 `window_to_show`가 고른 창을 보입니다.
  - Windows: 작은 창이 마지막 창이라 Alt+F4로 닫으면 앱이 종료됩니다(FR-WINDOW-007).
- **감지·훅 설치 화면은 `features/onboarding/` 하나로 합칩니다.** 세 단계가 감지 결과와 설치 상태를 함께 쓰기 때문입니다. 한 기능 안의 reducer로 다루고 store는 만들지 않습니다. 쓰는 곳이 시작 설정 창 하나뿐입니다.
- **고정 목록**
  - Claude: 별칭 5개
  - Codex: 이 Mac에서 조회한 목록 표시 모델 3개
  - 버린 방안: paseo처럼 전체 ID 목록을 계속 갱신하는 방식은 관리 비용이 큽니다.

### 위험

- [Claude Code가 초기화 응답 형식을 바꿈] → 해석에 실패하면 고정 목록으로 넘어갑니다(FR-START-007). 해석 테스트는 이번에 받은 실제 응답을 바탕으로 합니다.
- [초기화 요청에도 훅이 실행됨] → `disableAllHooks`를 넘기고, 검증 때 앱을 켠 채 조회해 훅 이벤트가 생기지 않는지 확인합니다.
- [Claude가 첫 실행 로그인이나 업데이트 안내로 멈춤] → 5초 상한 뒤 프로세스를 죽이고 고정 목록을 씁니다(SC-START-001).
- [Windows `claude.cmd`나 `codex.cmd` 같은 npm 셸 스크립트] → 감지의 `--version` 실행과 같은 방식으로 띄웁니다. 실패하면 고정 목록을 씁니다. Windows 실기 확인을 검증 항목에 넣습니다.
- [배율이 다른 모니터에서 크기나 위치가 틀어짐] → 숨긴 채 옮긴 뒤 크기를 정하고, Windows 두 모니터(100%, 150%) 실기 확인을 검증 항목에 넣습니다.
- [macOS의 Cmd+W가 테두리 없는 창에서 동작하지 않음] → Tauri 기본 메뉴의 창 닫기 항목에 기대며, 검증 때 확인합니다. 안 되면 앞 단계로 돌아가 스펙을 고칠지 묻습니다.
- [`macOSPrivateApi`로 Mac App Store 배포 불가] → clarify Q7에서 dmg로만 배포하기로 정했습니다.
- [투명 창 위 WebGL이 Windows에서 검은 배경으로 그려짐] → Canvas에 `gl={{ alpha: true }}`를 주고 배경색을 지정하지 않습니다. 검증 때 확인합니다.
- [시작 설정 창을 닫은 뒤 늦게 도착한 모델 조회 응답] → 창이 없어지면 응답은 버려지고 프로세스는 `kill_on_drop`으로 죽습니다.

## 작업

### 1. 모델 목록 core(Rust)

- [x] 1.1 `model/domain/model.rs` 타입과 `AgentKind`·`Environment`의 `Deserialize`. `pnpm test:rust`로 바인딩 생성 확인 (FR-START-004, FR-START-006)
- [x] 1.2 `parse.rs`의 `claude_models`, `codex_models`, `codex_config_model`과 테스트(실제 응답 예시, 숨김 모델 제외, 잘못된 입력) (FR-START-007)
- [x] 1.3 `catalog.rs`의 `fallback_models`, `model_list`, `parse_selection`, `selection_json`과 테스트(기본 모델, 목록에 없는 config 모델, 빈 조회 결과) (FR-START-004, FR-START-006, FR-START-007)

### 2. 모델 목록 조회 IO(Rust)

- [x] 2.1 `model/host.rs`의 `ModelHost`, `model/query.rs`의 `query_models`와 가짜 host 테스트(응답 없음 → 5초 뒤 fallback, 성공, Codex 설정 반영) (FR-START-007, SC-START-001)
- [x] 2.2 `platform/process.rs`에 표준 입력에 한 줄 쓰고 응답 줄을 기다리는 도우미. macOS와 Windows·WSL의 `ModelHost` 구현. 이 Mac에서 실제 조회가 되는지 확인 (FR-START-007)
- [x] 2.3 `commands/model.rs`의 `agent_models`. 감지 결과에서 경로를 찾아 조회하고 `lib.rs`에 등록 (FR-START-007)

### 3. 창 배치 core와 창 전환(Rust)

- [x] 3.1 `placement.rs`의 `pick_display`, `corner_rect`, `window_to_show`와 테스트(걸친 창, 모니터 밖, 200%·100% 배율, 작업 영역보다 큰 창, 독이나 작업 표시줄로 줄어든 작업 영역) (FR-WINDOW-003, FR-WINDOW-004, FR-WINDOW-005, FR-WINDOW-006)
- [x] 3.2 `state.rs`에 선택 보관과 전환 잠금. 선택을 넣고 읽는 테스트 (FR-START-006, FR-WINDOW-001)
- [x] 3.3 `window/main.rs`의 작은 창 만들기(숨김 → 위치·크기 → 보임, 투명, 항상 위, 테두리 없음, 작업 표시줄 표시) (FR-WINDOW-001, FR-WINDOW-003, FR-WINDOW-004, FR-WINDOW-005)
- [x] 3.4 `commands/onboarding.rs`의 `saved_selection`, `finish_onboarding`(저장 → 열기 → 시작 설정 창 `destroy`). 실패 시 시작 설정 창 유지 (FR-START-006, FR-WINDOW-001)
- [x] 3.5 `window/mod.rs`의 지금 쓰는 창 보이기. single-instance 콜백과 macOS 독 클릭을 이 함수로 바꿈 (FR-WINDOW-006, FR-HOOK-014)
- [x] 3.6 `tauri.conf.json`(`onboarding` 창, `macOSPrivateApi`), `Cargo.toml` feature, `capabilities/onboarding.json`, `capabilities/main.json`. `pnpm lint:rust` 통과 (FR-START-001, FR-WINDOW-001, FR-WINDOW-008)

### 4. 시작 설정 core(TS)

- [x] 4.1 `features/agent-detection/detectionView.ts`와 테스트를 `features/onboarding/domain/`으로 옮김. 기존 테스트 통과 (FR-AGENT-008)
- [x] 4.2 `hookEventsView.ts`의 설치 결과 부분을 `installView.ts`로 옮김. 이벤트 목록 코드와 그 테스트는 지움 (FR-HOOK-015)
- [x] 4.3 `steps.ts`의 `stepStatuses`와 테스트 (FR-START-002, FR-HOOK-015)
- [x] 4.4 `selection.ts`의 `agentChoices`, `choicesToQuery`, `selectionReducer`, `canProceed`, `toSelection`과 테스트(하나뿐인 항목, 설치된 항목 없음, 저장값 미리 고르기, 에이전트 바꾸면 기본 모델, 다시 감지 뒤 유지, 골라 둔 WSL 재조회, 로딩 중 다음 불가) (FR-START-003, FR-START-004, FR-START-005, FR-START-006, FR-START-007)

### 5. 화면과 3D(TS)

- [x] 5.1 `api/index.ts`에 `getAgentModels`, `getSavedSelection`, `finishOnboarding`, `startWindowDrag` (FR-START-006, FR-START-007, FR-WINDOW-008)
- [x] 5.2 `features/onboarding/`의 감지 단계, 훅 설치 단계, 선택 단계 컴포넌트와 `Onboarding`. 훅 설치 실패 표시, 모델 불러오는 중 표시, 다음 버튼 (FR-START-002, FR-START-003, FR-START-004, FR-START-005, FR-AGENT-008, FR-HOOK-015)
- [x] 5.3 `scene/TestScene.tsx`. 투명 배경 위에서 계속 도는 도형, 불러오는 중 표시 (FR-WINDOW-002)
- [x] 5.4 `features/window-drag/WindowDragArea.tsx` (FR-WINDOW-008)
- [x] 5.5 `features/versions`, `features/agent-detection`, `features/hook-events` 지우기와 `main.css` 정리. `pnpm lint`, `pnpm test` 통과 (FR-START-001)

### 6. 창 연결과 검증

- [x] 6.1 `onboarding.html`, `windows/onboarding/`과 `vite.config.ts` 입력 등록. `windows/main/`을 작은 창(`TestScene` + `WindowDragArea`)으로 바꿈. `pnpm build` 통과 (FR-START-001, FR-WINDOW-001, FR-WINDOW-002)
- [x] 6.2 macOS 실기 확인. 앱 실행, 세 단계, 다음, 오른쪽 아래 작은 창, 투명·항상 위, 끌기, Cmd+W 뒤 독 클릭, 다시 실행, 다음 실행 때 미리 골라 둠, 조회 중 훅 이벤트 없음, 전환 1초 이내 (FR-START-001~007, FR-WINDOW-001~008, FR-HOOK-014, SC-WINDOW-001, SC-START-001)
- [x] 6.3 Windows 실기 확인(사용자에게 요청). WSL 항목 고를 때 조회, 두 모니터·다른 배율, 작업 표시줄 위 배치, Alt+F4 종료, 투명 배경 (FR-START-007, FR-WINDOW-003, FR-WINDOW-004, FR-WINDOW-005, FR-WINDOW-007, FR-HOOK-011)
- [x] 6.4 PR CI와 같은 검사(`format:check`, `lint`, `test`, `build`, `format:check:rust`, `lint:rust`, `test:rust`)와 Windows 대상 `cargo clippy` (FR-HOOK-011)
