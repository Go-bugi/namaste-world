# Plan

spec-hash: 59cf978ff9f1d8e8

## 설계

### 층 배치

| 층 | 위치 | 내용 |
| --- | --- | --- |
| core(Rust) | `src-tauri/src/login/domain/model.rs` | `LoginStatus`, `LoginMethod`, `LoginFailure` 타입(ts-rs) |
| core(Rust) | `src-tauri/src/login/domain/parse.rs` | `claude auth status --json`, `codex login status` 출력을 `LoginStatus`로 해석 |
| core(Rust) | `src-tauri/src/agent/domain/assemble.rs` | 보관한 감지 결과에서 설치됨 항목의 실행 파일 경로를 찾는 `installed_path`. 지금 `commands/model.rs`에 있는 같은 계산을 옮겨 두 명령이 같이 쓴다 |
| 로직 모듈(Rust) | `src-tauri/src/login/host.rs`, `login/check.rs` | CLI 실행을 추상화한 `LoginHost` trait과, 시간 상한을 걸어 확인하는 `check_login` |
| IO(Rust) | `src-tauri/src/platform/{macos,windows,mod}.rs`, `platform/process.rs` | `LoginHost` 구현. 모델 조회용 `ModelCli`를 `AgentCli`로 이름을 바꿔 두 trait을 함께 구현한다(같은 PATH, WSL 실행 방식을 쓴다) |
| shell(Rust) | `src-tauri/src/commands/login.rs`, `commands/hook.rs`, `lib.rs` | `agent_login_status`, `reinstall_hooks` 명령과 등록 |
| IO(TS) | `src/renderer/src/api/index.ts` | `getAgentLoginStatus`, `reinstallHooks` |
| core(TS) | `src/renderer/src/features/onboarding/domain/stepper.ts` | 단계 순서, 단계 표시, 이전/다음 가능 여부. 지금의 `steps.ts`를 대신한다 |
| core(TS) | `features/onboarding/domain/login.ts` | 로그인 확인 상태 reducer, 로그인 안내 문구와 명령, 상태 문구 |
| core(TS) | `features/onboarding/domain/selection.ts`, `detectionView.ts` | 모델 목록 다시 불러오기 가능 여부, 다시 감지 가능 여부에 훅 설치 중 조건 추가 |
| React | `features/onboarding/` | `Onboarding.tsx`(지금 단계 상태와 이벤트 핸들러), `StepIndicator.tsx`, `StepNav.tsx`, `LoginStep.tsx`, 기존 `DetectionStep`, `InstallStep`, `SelectStep` 수정 |

로직 모듈 `login/`은 IO가 섞여 있어(trait으로 받는 `check.rs`) `login/domain/`을 둔다. `#[cfg(target_os)]` 코드는 `platform/`에만 둔다.

### 타입과 경계

Rust에 정의하고 ts-rs로 내보낸다(`pnpm test:rust`로 `types/bindings/` 생성).

```rust
// login/domain/model.rs
#[derive(Serialize, TS)] #[serde(tag = "kind", rename_all = "camelCase")] #[ts(export)]
pub enum LoginStatus {
    LoggedIn {
        #[ts(optional)] #[serde(skip_serializing_if = "Option::is_none")]
        method: Option<LoginMethod>,
    },
    LoggedOut,
    Failed { reason: LoginFailure },
}

#[derive(Serialize, TS)] #[serde(rename_all = "camelCase")] #[ts(export)]
pub enum LoginMethod { Subscription, ApiKey, Cloud, Other }

#[derive(Serialize, TS)] #[serde(tag = "kind", rename_all = "camelCase")] #[ts(export)]
pub enum LoginFailure {
    NotInstalled,      // 보관한 감지 결과에 그 항목이 설치됨으로 없음
    LaunchFailed,      // CLI를 띄우지 못함
    Timeout,           // SC-AGENT-002 상한 초과
    UnreadableOutput,  // 출력으로 로그인 여부를 판단할 수 없음
}
```

TS 모양: `LoginStatus = { kind: 'loggedIn'; method?: LoginMethod } | { kind: 'loggedOut' } | { kind: 'failed'; reason: LoginFailure }`.

새 Rust 명령:

| 명령 | 인자 | 반환 | 비고 |
| --- | --- | --- | --- |
| `agent_login_status` | `agent: AgentKind`, `environment: Environment` | `LoginStatus` | 실패하지 않는다. 문제는 `Failed`로 담는다 |
| `reinstall_hooks` | 없음 | `HookInstallation` | 보관한 감지 결과로 `tasks::hook_install::run`을 돌리고 끝나면 결과를 준다. 이벤트 `hook-installation`도 기존대로 나간다 |

새 이벤트와 capability 권한은 없다. 명령 복사는 웹 표준 `navigator.clipboard.writeText`를 쓰므로 Tauri 권한이 필요 없다.

### 시그니처

Rust core:

```rust
// login/domain/parse.rs
/// `claude auth status --json`의 stdout. 종료 코드와 상관없이 JSON의 `loggedIn`으로 판단한다.
/// loggedIn=true → LoggedIn(method: authMethod "claude.ai"면 Subscription, apiProvider가 "firstParty"가 아니면 Cloud,
/// authMethod에 "api"나 "console"이 들어 있으면 ApiKey, 그 밖은 Other. authMethod가 없거나 문자열이 아니면 method는 None), false → LoggedOut.
/// JSON이 아니거나 `loggedIn`이 bool이 아니면 None.
pub fn claude_status(stdout: &str) -> Option<LoginStatus>;

/// `codex login status`의 stdout과 stderr를 합친 글. 결과 문장은 stderr로 나온다.
/// "Logged in using ChatGPT" → LoggedIn(Subscription), "Logged in using an API key ..." → LoggedIn(ApiKey),
/// 그 밖의 "Logged in using ..." → LoggedIn(Other), "Not logged in" → LoggedOut, 나머지는 None.
/// API 키 뒤에 붙는 가린 키 문자열은 결과에 담지 않는다.
pub fn codex_status(stdout: &str, stderr: &str) -> Option<LoginStatus>;

// agent/domain/assemble.rs
/// 감지 결과에서 agent와 environment가 같고 설치됨인 항목의 실행 파일 경로. 없으면 None.
pub fn installed_path(detection: &Detection, agent: AgentKind, environment: &Environment) -> Option<String>;
```

Rust 로직 모듈:

```rust
// login/host.rs
pub struct CliOutput { pub stdout: String, pub stderr: String }

pub trait LoginHost: Sync {
    /// environment에서 path를 args로 실행하고 끝나면 출력을 준다. 종료 코드가 0이 아니어도 출력을 준다.
    /// 띄우지 못하면 None. future가 버려지면 프로세스 트리를 죽인다.
    fn run(&self, environment: &Environment, path: &str, args: &[&str]) -> impl Future<Output = Option<CliOutput>> + Send;
}
pub const CLAUDE_STATUS_ARGS: [&str; 3] = ["auth", "status", "--json"];
pub const CODEX_STATUS_ARGS: [&str; 2] = ["login", "status"];

// login/check.rs
pub const LOGIN_CHECK_LIMIT: Duration = Duration::from_secs(5); // SC-AGENT-002
/// limit 안에 끝나지 않으면 Failed(Timeout), 띄우지 못하면 Failed(LaunchFailed),
/// 해석하지 못하면 Failed(UnreadableOutput).
pub async fn check_login(host: &impl LoginHost, agent: AgentKind, environment: &Environment, path: &str, limit: Duration) -> LoginStatus;
```

TS core:

```ts
// domain/stepper.ts
export type StepId = 'detect' | 'install' | 'select' | 'login';
export const STEPS: readonly StepId[]; // 감지, 훅 설치, 선택, 로그인 순서
export type StepperInput = {
  readonly current: StepId;
  readonly detecting: boolean;        // 앱 시작 감지나 다시 감지 진행 중
  readonly hasInstalled: boolean;     // 감지 결과에 설치됨 항목이 하나 이상
  readonly installing: boolean;       // 훅 설치(다시 감지 뒤 설치, 다시 설치 포함) 진행 중
  readonly hasInstallation: boolean;
  readonly selectionReady: boolean;   // 에이전트와 모델이 모두 골라짐
  readonly loggedIn: boolean;         // 지금 고른 항목의 확인 결과가 로그인됨
  readonly submitting: boolean;       // 보관과 작은 창 전환 진행 중
};
/** 이전/다음 버튼 활성 여부. FR-START-005의 표를 그대로 따른다. */
export function navigation(input: StepperInput): { readonly canPrev: boolean; readonly canNext: boolean };
/** 단계 표시줄. 각 단계의 지금 단계 여부와 상태(pending | running | done).
 *  done: 감지는 detecting이 아님, 훅 설치는 installing이 아니고 hasInstallation, 선택은 selectionReady, 로그인은 loggedIn.
 *  done이 아니면 지금 단계이거나 그 단계의 일(감지, 설치)이 진행 중일 때 running, 그 밖은 pending. */
export function indicators(input: StepperInput): readonly { readonly id: StepId; readonly current: boolean; readonly status: StepStatus }[];
export function nextStep(current: StepId): StepId | undefined;
export function prevStep(current: StepId): StepId | undefined;

// domain/login.ts
export type LoginState =
  | { readonly kind: 'idle' }
  | { readonly kind: 'checking'; readonly key: string; readonly requestId: number }
  | { readonly kind: 'checked'; readonly key: string; readonly status: LoginStatus };
export type LoginEvent =
  | { readonly type: 'checkStarted'; readonly key: string; readonly requestId: number }
  | { readonly type: 'checked'; readonly requestId: number; readonly status: LoginStatus };
/** requestId가 지금 확인과 다른 결과(이전 항목, 이전 확인)는 버린다. */
export function loginReducer(state: LoginState, event: LoginEvent): LoginState;
/** key 항목의 결과가 로그인됨인가. */
export function isLoggedIn(state: LoginState, key: string | undefined): boolean;
/** 다시 확인 버튼을 누를 수 있는가. 확인 중이면 false. */
export function canRecheck(state: LoginState): boolean;
/** 로그인 안 됨일 때만 안내. command는 Claude `claude auth login`, Codex `codex login`.
 *  WSL이면 where에 배포판 이름이 든 "WSL <배포판> 터미널" 문구. */
export function loginGuide(agent: AgentKind, environment: Environment, status: LoginStatus): { readonly command: string; readonly where: string } | undefined;
export function loginStatusLabel(status: LoginStatus): string;   // 로그인됨(방식), 로그인 안 됨, 확인 실패: 이유

// domain/selection.ts
/** 모델 목록 다시 불러오기 버튼. 에이전트를 골랐고 그 목록이 불러오는 중이 아닐 때만 true. */
export function canReloadModels(state: SelectionState): boolean;
// domain/detectionView.ts (시그니처 변경)
export function canRedetect(state: PanelState, installing: boolean): boolean;
```

`canProceed`는 `navigation`이 대신하므로 지운다. 선택 단계 조건은 `toSelection(state) !== undefined`를 `selectionReady`로 넘긴다.

### 결정

- **로그인 상태는 CLI의 상태 명령으로 묻는다**(`claude auth status --json`, `codex login status`). 파일과 키체인을 읽지 않아 FR-AGENT-011을 지키고, CLI가 저장 형식을 바꿔도 깨지지 않는다. Claude 초기화 응답의 계정 정보를 쓰는 방법은 모델 조회와 묶이지만 훅을 끄는 설정과 stream-json이 필요해 더 무겁다.
- **종료 코드로 판단하지 않는다.** 두 CLI 모두 로그인 안 됨에서 종료 코드 1을 내므로 출력 내용으로 판단한다. Codex는 결과를 stderr로 내서 stdout과 stderr를 함께 받는다(2026-11-17 Claude Code 2.1.326, codex-cli 0.146.0에서 확인).
- **`ModelCli`를 `AgentCli`로 바꿔 `ModelHost`와 `LoginHost`를 함께 구현한다.** macOS 로그인 셸 PATH와 WSL 실행(`wsl_exec`)을 그대로 쓴다. 따로 구조체를 만들면 같은 실행 코드가 두 벌이 된다.
- **로그인 확인은 이벤트 핸들러에서 시작한다.** 선택 단계에서 다음을 누를 때와 다시 확인을 누를 때 `api.getAgentLoginStatus`를 부른다. 단계 진입을 `useEffect`로 감시하지 않는다.
- **진행 상태는 지금처럼 컴포넌트가 들고, 판단은 `stepper.ts`의 순수 함수가 한다.** `navigation`과 `indicators`는 입력만 보고 결정하므로 FR-START-005 시나리오를 그대로 테스트로 옮길 수 있다.
- **다시 설치의 진행 중 표시는 기존 설치 카운트(`pendingInstalls`)를 쓴다.** 누를 때 1 늘리고 `hook-installation` 이벤트로 줄인다. 명령이 IPC로 실패하면 바로 줄인다.
- **명령 복사는 `navigator.clipboard.writeText`.** 클립보드 플러그인과 권한을 추가하지 않는다.

### 위험

- [Claude Code의 `authMethod` 값을 모두 알지 못한다] → 모르는 값은 `Other`로 두고 로그인됨으로 낸다. 로그인 판단은 `loggedIn`만 본다.
- [오래된 CLI에 상태 명령이나 `--json`이 없다] → 출력을 해석하지 못해 확인 실패가 되고 넘어갈 수 없다. 실패 문구에 CLI 업데이트를 안내한다.
- [`codex login status`가 가린 API 키를 출력한다] → 해석할 때 버리고 결과와 로그에 담지 않는다(FR-AGENT-011).
- [WKWebView에서 `navigator.clipboard.writeText`가 거부될 수 있다] → 버튼 클릭 핸들러 안에서 부르고, 실패하면 복사되었음 표시를 하지 않는다. 명령 글자는 선택할 수 있게 둔다.
- [WSL 셸 래퍼의 `timeout -s KILL 6`이 5초 상한보다 길다] → 바깥에서 `check_login`이 5초에 끊고 future를 버려 프로세스 트리를 죽인다.

## 작업

### 1. 로그인 상태 해석(Rust core)

- [x] 1.1 `login/domain/model.rs`에 `LoginStatus`, `LoginMethod`, `LoginFailure`를 정의하고 `login` 모듈을 `lib.rs`에 등록한다. 확인: `cargo check` (FR-AGENT-009, FR-AGENT-010)
- [x] 1.2 `test-writer`가 `claude_status`, `codex_status` 테스트를 쓴다(로그인됨 구독/API 키/클라우드/모르는 방식, 로그인 안 됨, 해석 불가, 가린 키 미포함). 확인: 테스트가 실패하는 상태로 커밋 가능 (FR-AGENT-009, FR-AGENT-010, FR-AGENT-011)
- [x] 1.3 `login/domain/parse.rs`를 구현한다. 확인: `pnpm test:rust` 통과 (FR-AGENT-009, FR-AGENT-010, FR-AGENT-011)
- [x] 1.4 `test-writer`가 `installed_path` 테스트를 쓰고, `agent/domain/assemble.rs`에 구현한 뒤 `commands/model.rs`가 이 함수를 쓰게 바꾼다. 확인: `pnpm test:rust` 통과 (FR-AGENT-009)

### 2. 로그인 확인 실행(Rust 로직 모듈과 platform)

- [x] 2.1 `login/host.rs`에 `LoginHost`, `CliOutput`, 인자 상수를 둔다. 확인: `cargo check` (FR-AGENT-009)
- [x] 2.2 `test-writer`가 가짜 `LoginHost`로 `check_login` 테스트를 쓴다(에이전트별 인자와 경로·환경 전달, 시간 초과 5초, 띄우지 못함, 해석 불가). 확인: 테스트 작성 (FR-AGENT-009, FR-AGENT-010, SC-AGENT-002)
- [x] 2.3 `login/check.rs`를 구현한다. 확인: `pnpm test:rust` 통과 (FR-AGENT-009, FR-AGENT-010, SC-AGENT-002)
- [x] 2.4 `platform/process.rs`에 stderr까지 받는 트리 실행을 추가하고, `ModelCli`를 `AgentCli`로 바꿔 macOS, Windows(WSL 포함), 지원하지 않는 OS에서 `LoginHost`를 구현한다. 확인: `pnpm lint:rust`, `pnpm test:rust` 통과 (FR-AGENT-009, FR-AGENT-011)

### 3. 명령과 TS 경계

- [x] 3.1 `commands/login.rs`에 `agent_login_status`를 만들고 등록한다. 감지 결과에 없으면 `Failed(NotInstalled)`. `pnpm test:rust`로 바인딩을 만든다. 확인: `types/bindings/LoginStatus.ts` 등 생성 (FR-AGENT-009, FR-AGENT-011)
- [x] 3.2 `commands/hook.rs`에 `reinstall_hooks`를 만들고 등록한다. 확인: `pnpm lint:rust` 통과, 앱에서 호출 시 `hook-installation` 이벤트 수신 (FR-HOOK-016)
- [x] 3.3 `api/index.ts`에 `getAgentLoginStatus`, `reinstallHooks`를 추가한다. 확인: `pnpm typecheck` (FR-AGENT-009, FR-HOOK-016)

### 4. 시작 설정 화면의 계산(TS core)

- [x] 4.1 `test-writer`가 `stepper.ts`의 `navigation`, `indicators`, `nextStep`, `prevStep` 테스트를 FR-START-002, FR-START-005 시나리오대로 쓴다(감지 중, 설치된 에이전트 없음, 훅 설치 중 두 버튼 비활성화, 선택 미완료, 로그인 안 됨/확인 실패/확인 중, 로그인됨, 보관 중, 감지 단계의 이전 버튼). 확인: 테스트 작성 (FR-START-002, FR-START-005)
- [x] 4.2 `stepper.ts`를 구현하고 `steps.ts`와 그 테스트를 지운다. 확인: `pnpm test` 통과 (FR-START-002, FR-START-005)
- [x] 4.3 `test-writer`가 `login.ts` 테스트를 쓴다(늦게 온 결과 버림, 다시 확인 가능 여부, 에이전트별 명령, WSL 안내, 로그인됨이면 안내 없음, 상태 문구). 확인: 테스트 작성 (FR-START-008, FR-START-009)
- [x] 4.4 `login.ts`를 구현한다. 확인: `pnpm test` 통과 (FR-START-008, FR-START-009)
- [x] 4.5 `test-writer`가 `canReloadModels`와 `canRedetect(state, installing)` 테스트를 쓰고, 구현한 뒤 `canProceed`를 지운다. 확인: `pnpm test` 통과 (FR-START-010, FR-AGENT-008)

### 5. 시작 설정 화면 연결(React)

- [x] 5.1 `StepIndicator.tsx`, `StepNav.tsx`를 만들고 `Onboarding.tsx`가 지금 단계 하나만 보여 주도록 바꾼다. 다음과 이전은 `navigation` 결과로 비활성화하고, 마지막 단계의 다음은 `finishOnboarding`을 부른다. 확인: `pnpm dev`에서 네 단계를 버튼으로 오가고, 감지 중·설치 중에 두 버튼이 꺼짐 (FR-START-002, FR-START-005, FR-START-006)
- [x] 5.2 `DetectionStep`의 다시 감지 버튼을 `canRedetect(panel, installing)`로 바꾼다. 확인: 다시 감지 뒤 설치가 끝날 때까지 버튼이 꺼짐 (FR-AGENT-008)
- [x] 5.3 `InstallStep`에 다시 설치 버튼을 넣고 `reinstallHooks`와 설치 카운트로 연결한다. 확인: 누르면 진행 중이 보이고 끝나면 결과가 바뀜, 그동안 버튼과 이전·다음이 꺼짐 (FR-HOOK-016, FR-START-005)
- [x] 5.4 `SelectStep`에서 자체 다음 버튼을 빼고, 모델 목록 다시 불러오기 버튼을 넣어 고른 항목만 다시 조회한다. 확인: 누르면 불러오는 중이 보이고 고른 모델이 유지됨 (FR-START-010)
- [x] 5.5 `LoginStep.tsx`를 만들고, 선택 단계에서 다음을 누를 때와 다시 확인을 누를 때 로그인 확인을 시작한다. 로그인 안 됨이면 명령과 복사 버튼, WSL 안내를 보여 준다. 확인: `pnpm dev`에서 로그인된 Claude Code는 로그인됨과 방식이 보이고 다음이 켜짐. 빈 `CLAUDE_CONFIG_DIR`로 앱을 띄우면 로그인 안 됨과 `claude auth login` 안내가 보이고 복사가 됨 (FR-START-008, FR-START-009, FR-AGENT-009, FR-AGENT-011)

### 6. 검증

- [x] 6.1 PR CI와 같은 명령을 돌린다: `pnpm format:check`, `pnpm lint`, `pnpm test`, `pnpm build`, `pnpm format:check:rust`, `pnpm lint:rust`, `pnpm test:rust`, `pnpm sdd:check`. 확인: 모두 통과 (FR-AGENT-009, FR-START-005)
- [x] 6.2 로그인 확인이 5초 안에 끝나는지와 확인 중 키체인 권한 창이 뜨지 않는지 앱에서 확인한다. 확인: macOS에서 로그인 확인 단계 진입부터 결과까지 5초 이내, 권한 창 없음 (SC-AGENT-002, FR-AGENT-011)
