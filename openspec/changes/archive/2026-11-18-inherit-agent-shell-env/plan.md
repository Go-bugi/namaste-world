# Plan

spec-hash: ee298d5a9a422d6b

## 설계

### 층 배치

| 층 | 위치 | 내용 |
| --- | --- | --- |
| core(Rust) | `src-tauri/src/shell/model.rs` | `ShellKind`, `ShellSetup`, `ShellEnvFailure`(ts-rs), 메모리 전용 `ShellEnv` |
| core(Rust) | `src-tauri/src/shell/env.rs` | 셸 출력에서 환경 꺼내기(`env -0`, PowerShell JSON), 앱 환경과 합치기, 설정 폴더 계산 |
| core(Rust) | `src-tauri/src/shell/choice.rs` | 셸 선택 파일 읽기/쓰기 문자열, 실제로 쓸 셸 정하기, 선택지 목록, Git Bash 후보 경로 |
| core(Rust) | `src-tauri/src/shell/wsl.rs` | WSL 배포판 사용자의 로그인 셸로 명령을 실행하는 `sh` 스크립트 만들기, 셸 설정 파일이 찍은 출력을 걸러 내는 경계 처리 |
| core(Rust) | `src-tauri/src/agent/domain/model.rs` | `Detection`에 `shell_env_failure` 추가(FR-AGENT-018) |
| core(Rust) | `src-tauri/src/hook/domain/model.rs`, `hook/install.rs` | `InstallTarget`에 `claude_dir`, `codex_dir`를 두고 설정 파일 경로를 홈 대신 이 값으로 만든다 |
| IO(Rust) | `src-tauri/src/platform/process.rs` | 띄우기 실패와 시간 초과를 구분해 돌려주는 `run_checked` |
| IO(Rust) | `src-tauri/src/platform/macos.rs` | `$SHELL -ilc`로 환경 전체 읽기. `AgentHost`, `AgentCli`, `hook_target`이 `ShellEnv`를 받는다. `static SHELL_PATH`는 없앤다 |
| IO(Rust) | `src-tauri/src/platform/windows.rs` | PowerShell(7 우선)/Git Bash로 환경 읽기, Git Bash 찾기. WSL 명령을 `shell::wsl` 스크립트로 감싼다. 훅 대상 폴더와 Codex 설정을 WSL 로그인 셸에서 읽는다 |
| IO(Rust) | `src-tauri/src/platform/mod.rs` | `read_shell_env`, `git_bash_available`, `DETECT_AT_STARTUP` 상수를 OS별로 내보낸다 |
| 공유 상태(Rust) | `src-tauri/src/state.rs` | `ShellEnvState`: 마지막으로 읽은 `ShellEnv`와 그 셸. 디스크에 쓰지 않는다 |
| shell(Rust) | `src-tauri/src/tasks/agent_detection.rs` | 셸 환경을 읽고 남은 시간으로 감지하고, 실패를 `Detection`에 담는다 |
| shell(Rust) | `src-tauri/src/commands/shell.rs`, `commands/agent.rs`, `commands/model.rs`, `commands/login.rs`, `lib.rs` | `shell_setup`, `set_shell`, `detect_with_shell` 명령. 모델 조회와 로그인 확인은 `ShellEnvState`의 환경을 쓴다. Windows는 시작 감지를 띄우지 않는다 |
| IO(TS) | `src/renderer/src/api/index.ts` | `getShellSetup`, `setShell`, `detectWithShell` |
| core(TS) | `src/renderer/src/features/onboarding/domain/stepper.ts` | 셸 단계를 넣은 단계 목록과 이전/다음 조건 |
| core(TS) | `features/onboarding/domain/shell.ts` | 셸 선택지 문구, 다음을 누를 때 감지할지 판단 |
| core(TS) | `features/onboarding/domain/detectionView.ts` | 셸 환경 읽기 실패 안내 문구 |
| React | `features/onboarding/ShellStep.tsx`, `Onboarding.tsx`, `DetectionStep.tsx` | 셸 선택 단계, 감지 시작 시점, 실패 안내 |

`shell/`은 IO가 없는 모듈이라 `domain/`을 두지 않는다. 셸을 실제로 띄우는 코드는 OS마다 달라 `platform/`에 둔다. `#[cfg(target_os)]`는 `platform/`에만 쓴다.

### 타입과 경계

ts-rs로 내보내는 타입(`pnpm test:rust`로 `types/bindings/` 생성):

```rust
// shell/model.rs
#[derive(Serialize, Deserialize, TS, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")] #[ts(export)]
pub enum ShellKind { PowerShell, Cmd, GitBash }

/// Windows 셸 선택 단계에 보여 줄 것. macOS에서는 명령이 None을 돌려준다.
#[derive(Serialize, TS)] #[ts(export)]
pub struct ShellSetup { pub options: Vec<ShellKind>, pub selected: ShellKind }

/// 셸 환경을 읽지 못한 이유(FR-AGENT-018). 문자열을 담지 않아 환경 값이 섞일 수 없다.
#[derive(Serialize, TS, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")] #[ts(export)]
pub enum ShellEnvFailure { Timeout, LaunchFailed, ExitedWithError, UnreadableOutput }

// agent/domain/model.rs
pub struct Detection {
    pub entries: Vec<AgentEntry>,
    #[ts(optional)] #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_env_failure: Option<ShellEnvFailure>,
}
```

메모리 전용 타입(FR-AGENT-017). `Serialize`를 붙이지 않고 `Debug`는 값을 가린다.

```rust
/// CLI를 실행할 환경. 셸에서 읽은 값과 앱이 받은 값을 합친 것.
pub struct ShellEnv { vars: BTreeMap<String, String> }
impl ShellEnv {
    pub fn vars(&self) -> impl Iterator<Item = (&str, &str)>;
    pub fn get(&self, key: &str) -> Option<&str>;
}
impl fmt::Debug for ShellEnv { /* 키 개수만 찍는다 */ }
```

새 Rust 명령:

| 명령 | 인자 | 반환 | 비고 |
| --- | --- | --- | --- |
| `shell_setup` | 없음 | `Option<ShellSetup>` | macOS는 None. Windows는 Git Bash 감지 결과와 저장한 선택으로 만든다 |
| `set_shell` | `shell: ShellKind` | `()` | 앱 데이터 폴더 `shell.json`에 바로 저장한다(FR-START-012). 감지는 하지 않는다 |
| `detect_with_shell` | `shell: ShellKind` | `Detection` | 첫 호출은 앱 시작 감지로 publish한다(훅 설치는 기존 시작 설치 작업이 이어서 함). 마지막 감지와 셸이 같으면 보관한 결과를 준다. 다르면 `Trigger::Startup`(멈춘 배포판을 켜지 않음)으로 다시 감지하고 훅 설치를 띄운다 |

`redetect_agents`는 그대로 두되, Windows에서는 `ShellEnvState`의 마지막 셸로 감지한다. 새 이벤트와 capability 권한은 없다. 셸 환경 값은 어떤 명령 결과와 이벤트에도 실리지 않는다.

### 시그니처

Rust core(`shell/`):

```rust
// env.rs
/// `printf '\0<m>\0'; env -0; printf '\0<m>\0'` 출력에서 환경을 꺼낸다. 마커 앞뒤의 셸 설정 출력은 무시한다.
/// 마커가 두 번 없거나 `KEY=VALUE`가 아닌 항목만 있으면 None. `=` 없는 항목은 버린다.
pub fn parse_env_nul(stdout: &[u8], marker: &str) -> Option<BTreeMap<String, String>>;

/// `<m>{"KEY":"VALUE",...}<m>` 형태의 PowerShell 출력에서 환경을 꺼낸다. JSON이 아니거나 값이 문자열이 아니면 None.
pub fn parse_env_json(stdout: &str, marker: &str) -> Option<BTreeMap<String, String>>;

/// 앱이 받은 환경 위에 셸 환경을 덮는다. 앱에만 있는 변수는 남는다(FR-AGENT-012).
/// `case_insensitive`(Windows)면 이름을 대소문자 구분 없이 같은 변수로 보고 셸 쪽 이름을 쓴다.
pub fn merge_env(app: &BTreeMap<String, String>, shell: &BTreeMap<String, String>, case_insensitive: bool) -> ShellEnv;

/// 에이전트 설정 폴더. Claude는 `CLAUDE_CONFIG_DIR`, Codex는 `CODEX_HOME`이 비어 있지 않으면 그 값,
/// 아니면 `home` + `separator` + `.claude`/`.codex`(FR-HOOK-002).
pub fn config_dir(env: &ShellEnv, agent: AgentKind, home: &str, separator: char) -> String;

/// 셸을 한 번 실행한 결과. `platform/`이 `run_checked` 결과에서 만든다.
pub enum ShellRun { LaunchFailed, Timeout, Exited { success: bool, stdout: Vec<u8> } }

/// 셸 실행 결과를 CLI 실행 환경과 실패 이유로 바꾼다(FR-AGENT-016, FR-AGENT-018).
/// 띄우기 실패 → LaunchFailed, 시간 초과 → Timeout, 0이 아닌 종료 → ExitedWithError, `parse`가 None → UnreadableOutput.
/// 실패하면 앱 환경만 담은 `ShellEnv`를 준다. 성공하면 `merge_env(app, 읽은 환경, case_insensitive)`다.
pub fn shell_env_outcome(
    app: &BTreeMap<String, String>,
    run: ShellRun,
    parse: impl Fn(&[u8]) -> Option<BTreeMap<String, String>>,
    case_insensitive: bool,
) -> (ShellEnv, Option<ShellEnvFailure>);

/// Windows에서 이 셸의 환경을 셸을 띄워 읽는가. 명령 프롬프트는 앱 환경을 그대로 써서 false(FR-AGENT-014).
pub fn reads_shell(shell: ShellKind) -> bool;

/// 셸 환경을 읽은 뒤 감지에 남은 시간. `total - elapsed`이고 0 아래로 내려가지 않는다(SC-AGENT-001, SC-AGENT-003).
pub fn remaining_limit(total: Duration, elapsed: Duration) -> Duration;

// choice.rs
pub fn parse_shell_choice(contents: &str) -> Option<ShellKind>;      // `shell.json`. 모르는 값이면 None
pub fn shell_choice_json(shell: ShellKind) -> String;
/// 저장한 값이 없으면 PowerShell, Git Bash인데 없으면 PowerShell(FR-START-012).
pub fn effective_shell(saved: Option<ShellKind>, git_bash_available: bool) -> ShellKind;
/// [PowerShell, Cmd] 뒤에 Git Bash가 있으면 GitBash(FR-START-011).
pub fn shell_options(git_bash_available: bool) -> Vec<ShellKind>;
/// Git for Windows bash.exe 후보. `ProgramFiles`, `ProgramW6432`, `ProgramFiles(x86)` 아래 `Git\bin\bash.exe`,
/// `LOCALAPPDATA\Programs\Git\bin\bash.exe`, `PATH`의 `...\Git\cmd`에 대응하는 `...\Git\bin\bash.exe` 순서. 중복 없음.
pub fn git_bash_candidates(program_files: &[&str], local_app_data: Option<&str>, path_env: &str) -> Vec<String>;

// wsl.rs
/// 배포판 안에서 `sh -c`로 돌릴 스크립트. `getent passwd`로 찾은 로그인 셸이 bash/zsh면 `-ilc`, 그 밖이거나 못 찾으면
/// `/bin/sh -lc`로 `inner`를 실행한다(FR-AGENT-013). `inner`는 stdout과 stderr에 `fence_marker`를 한 줄 찍은 뒤 실제 명령을 exec한다.
pub fn login_shell_script(inner: &str, fence_marker: &str) -> String;
/// 경계 마커 줄 뒤의 출력. 마커가 없으면 None(셸이 명령까지 가지 못함).
pub fn after_fence<'a>(output: &'a str, fence_marker: &str) -> Option<&'a str>;
```

Rust IO(`platform/`, 테스트는 core와 가짜 trait으로):

```rust
/// 고른 셸(macOS는 무시)로 환경을 읽고 앱 환경과 합친다. 실패하면 앱 환경과 이유를 준다(FR-AGENT-016). 5초 상한(SC-AGENT-003).
pub async fn read_shell_env(shell: ShellKind) -> (ShellEnv, Option<ShellEnvFailure>);
pub fn git_bash_available() -> bool;               // macOS는 false
pub const DETECT_AT_STARTUP: bool;                  // macOS true, Windows false(FR-AGENT-007)

// process.rs
pub enum RunError { LaunchFailed, Timeout }
pub async fn run_checked(command: Command, limit: Duration) -> Result<Output, RunError>;
```

공유 상태(`state.rs`, 테스트는 상태 값으로):

```rust
/// 마지막으로 읽은 셸 환경과 그 셸. 감지가 끝날 때만 바뀐다. 디스크에 쓰지 않는다(FR-AGENT-015, FR-AGENT-017).
pub struct ShellEnvState { /* Mutex<Option<(ShellKind, ShellEnv)>> */ }
impl ShellEnvState {
    pub fn new() -> Self;
    pub fn set(&self, shell: ShellKind, env: ShellEnv);
    /// 마지막으로 읽은 환경. 아직 없으면 `app`(앱이 받은 환경)을 준다. 셸을 띄우지 않는다.
    pub fn env_or(&self, app: impl FnOnce() -> ShellEnv) -> ShellEnv;
    /// 마지막 감지에 쓴 셸. 아직 감지하지 않았으면 None.
    pub fn shell(&self) -> Option<ShellKind>;
}
```

모델 조회와 로그인 확인은 `AgentCli::new(env: ShellEnv)`로 만들고, 명령이 `ShellEnvState::env_or`로 꺼낸 환경을 넘긴다.

`tasks/agent_detection::run(trigger, shell) -> (Detection, ShellEnv)`: 셸 환경을 읽은 시간을 `DETECTION_LIMIT`에서 빼고 감지한다(SC-AGENT-001). `ShellEnvState`에 환경과 셸을 넣고 `Detection.shell_env_failure`를 채운다.

TS core:

```ts
// stepper.ts
export type StepId = 'shell' | 'detect' | 'install' | 'select' | 'login';
/** 셸 단계가 있으면(Windows) 맨 앞에 'shell'을 둔다. FR-START-002 */
export function stepsFor(hasShellStep: boolean): readonly StepId[];
// StepperInput에 steps: readonly StepId[], shellConfirmed: boolean 추가.
// navigation: 'shell'의 canNext는 항상 true(진행 중 작업이 없을 때), canPrev는 steps[0]에서만 false. FR-START-005
// indicators: steps 순서로 낸다. 'shell'은 shellConfirmed면 done.
export function nextStep(steps: readonly StepId[], current: StepId): StepId | undefined;
export function prevStep(steps: readonly StepId[], current: StepId): StepId | undefined;

// shell.ts
export function shellLabel(shell: ShellKind): string;        // 'PowerShell' | '명령 프롬프트' | 'Git Bash'
export function shellDescription(shell: ShellKind): string;  // 셸 설정 파일의 환경 변수가 전달되는지 한 줄(FR-START-011)
/** 다음을 눌렀을 때 감지를 부를지. 아직 감지하지 않았거나 마지막 감지와 셸이 다르면 true. FR-START-013 */
export function shouldDetect(lastDetected: ShellKind | undefined, selected: ShellKind): boolean;

// detectionView.ts
/** 감지 결과의 셸 환경 실패 안내. 실패가 없으면 undefined. 이유 문구만 담고 환경 값은 담지 않는다. FR-AGENT-018 */
export function shellEnvNotice(detection: Detection): string | undefined;
```

### 문구

| 대상 | 문구 |
| --- | --- |
| `shellEnvNotice` | `터미널 설정을 읽지 못해 터미널과 결과가 다를 수 있습니다 (<이유>)` |
| 이유 | timeout `시간 초과`, launchFailed `셸을 열지 못함`, exitedWithError `셸 설정 오류`, unreadableOutput `결과를 읽지 못함` |
| `shellDescription` | powerShell `터미널의 PowerShell과 같은 설정으로 실행합니다`, cmd `Windows 기본 설정으로만 실행합니다`, gitBash `터미널의 Git Bash와 같은 설정으로 실행합니다` |

### 결정

- **macOS는 환경 전체를 `env -0`으로 읽습니다.** `PATH`만 마커로 찍던 방식을 넓혔습니다. NUL 구분이라 값에 줄바꿈이 있어도 깨지지 않습니다. 변수를 골라 넘기면 목록에 없는 변수(`ANTHROPIC_BASE_URL`, 프록시 등)에서 같은 문제가 다시 생겨서 쓰지 않습니다.
- **셸 환경은 감지 때만 읽고 `ShellEnvState`에 둡니다(FR-AGENT-015).** 모델 조회와 로그인 확인은 상태에서 꺼내 씁니다. 지금 `AgentCli::new`가 static에서 `PATH`를 읽는 방식을 상태로 옮긴 것입니다.
- **CLI에는 `Command::envs`로 합친 환경을 넘깁니다.** 인자(argv)로 넘기면 다른 프로세스가 프로세스 목록에서 볼 수 있어서 쓰지 않습니다.
- **Git Bash 후보 경로는 대소문자와 끝 구분자를 가리지 않습니다.** Windows 경로는 대소문자를 구분하지 않으므로 `PATH`의 `git\CMD`, `Git\cmd\`와 끝에 `\`가 붙은 `ProgramFiles`도 후보로 보고, 구분자는 하나만 남깁니다. 중복은 대소문자를 무시하고 없애며 먼저 나온 표기를 남깁니다.
- **WSL 로그인 셸 스크립트는 문자열과 실제 실행으로 확인합니다.** macOS에는 `getent`가 없어 `/bin/sh -c`로 돌리면 대체 경로(`/bin/sh -lc`)를 탑니다. 이 경로에서 경계 마커와 작은따옴표가 든 명령이 그대로 실행되는지 봅니다.
- **Windows PowerShell은 `pwsh.exe`를 먼저 띄워 보고, 띄우지 못하면 `powershell.exe`를 씁니다(clarify Q2).** Git Bash는 `bash -ilc` 안에서 `powershell.exe -NoProfile`로 환경을 JSON으로 찍습니다. 그래야 Git Bash의 MSYS 경로(`/c/...`)가 아니라 Windows 프로그램이 받는 모양의 값이 나옵니다(Orca와 같은 방식).
- **WSL은 명령마다 로그인 셸 래퍼로 실행합니다.** 환경을 한 번 읽어 넘기는 방식은 `wsl.exe` 인자나 `WSLENV`로 키를 넘겨야 해서 버렸습니다. 대화형 셸 설정이 stdout에 찍는 문구는 경계 마커로 걸러 냅니다.
- **Windows 첫 감지는 `detect_with_shell`이 합니다.** `lib.rs`의 시작 감지는 `platform::DETECT_AT_STARTUP`이 true일 때만 띄웁니다. 시작 훅 설치 작업은 지금처럼 첫 감지 결과를 기다리므로 그대로 둡니다.
- **셸을 바꿔 다시 감지할 때는 `Trigger::Startup`을 씁니다.** 그래서 멈춘 배포판을 켜지 않습니다(clarify Q8).
- **훅 설정 폴더는 `InstallTarget`에 계산된 값으로 넣습니다.** `install.rs`는 홈 대신 이 값을 씁니다. 훅 스크립트는 지금처럼 홈의 `.namaste-world`에 둡니다. 이전 폴더는 건드리지 않습니다(clarify Q7).

### 위험

- [`.zshrc`, `.bashrc`가 느려 5초를 넘김] → 앱 환경으로 진행하고 감지 단계에 안내합니다(FR-AGENT-016, FR-AGENT-018). 감지 남은 시간은 10초에서 뺀 값으로 줍니다.
- [대화형 셸이 터미널을 차지하거나 입력을 기다림] → macOS는 지금처럼 새 프로세스 그룹으로 띄우고 stdin을 닫습니다. 시간 초과 때 그룹째 죽입니다.
- [WSL 로그인 셸 래퍼 때문에 CLI 실행마다 셸 시작 시간이 더해짐] → 로그인 확인(5초), 모델 조회 상한 안에서 끝나는지 실제 배포판에서 확인합니다. 넘으면 기존처럼 확인 실패나 고정 목록으로 떨어집니다.
- [셸 설정 파일의 출력이 stdout에 섞여 JSON 파싱이 깨짐] → 경계 마커 뒤만 읽습니다(`after_fence`). Claude 초기화 응답은 지금도 줄마다 골라 읽습니다.
- [API 키가 로그나 에러로 샘] → `ShellEnv`는 `Serialize`가 없고 `Debug`가 값을 가립니다. 실패 이유는 문자열 없는 enum입니다. 테스트로 확인합니다.
- [Windows 환경 변수 이름의 대소문자(`Path`/`PATH`)] → `merge_env`가 대소문자를 구분하지 않고 합칩니다.
- [Git Bash가 `PATH`에만 있는 휴대용 설치] → `PATH`의 `Git\cmd`에서 `Git\bin\bash.exe`를 유추합니다. 그래도 못 찾으면 선택지에 나오지 않습니다(FR-START-011).

## 작업

### 1. 셸 환경 core(Rust)

- [x] 1.1 `shell/model.rs`에 `ShellKind`, `ShellSetup`, `ShellEnvFailure`, `ShellEnv`를 정의하고 `Debug`가 값을 찍지 않는 테스트를 둡니다. `pnpm test:rust`로 바인딩을 만듭니다 (FR-AGENT-017, FR-AGENT-018)
- [x] 1.2 `shell/env.rs`의 `parse_env_nul`, `parse_env_json`, `merge_env` 테스트를 test-writer가 쓰고 구현합니다. 셸 설정 출력이 앞뒤에 섞인 경우, 대소문자만 다른 Windows 변수 이름을 포함합니다 (FR-AGENT-012, FR-AGENT-014)
- [x] 1.3 `config_dir` 테스트(값 있음, 빈 값, 없음, Windows 구분자)를 쓰고 구현합니다 (FR-HOOK-002, FR-START-004)
- [x] 1.4 `shell/choice.rs`의 `parse_shell_choice`, `shell_choice_json`, `effective_shell`, `shell_options`, `git_bash_candidates` 테스트를 쓰고 구현합니다 (FR-START-011, FR-START-012)
- [x] 1.5 `shell/wsl.rs`의 `login_shell_script`, `after_fence` 테스트를 쓰고 구현합니다. 스크립트가 bash/zsh는 `-ilc`, 그 밖은 `/bin/sh -lc`로 가는지 문자열로 확인합니다 (FR-AGENT-013)

- [x] 1.6 `shell/env.rs`의 `shell_env_outcome`, `reads_shell`, `remaining_limit`와 `state.rs`의 `ShellEnvState` 테스트를 test-writer가 쓰고 구현합니다 (FR-AGENT-014, FR-AGENT-015, FR-AGENT-016, FR-AGENT-018, SC-AGENT-003)

### 2. 셸 환경 읽기와 감지(Rust IO)

- [x] 2.1 `process::run_checked`를 추가하고, 띄우기 실패와 시간 초과를 가르는지 macOS에서 없는 실행 파일과 `sleep`으로 확인합니다 (FR-AGENT-016, SC-AGENT-003)
- [x] 2.2 macOS `read_shell_env`: `$SHELL -ilc`로 `env -0`을 마커 사이에 찍어 읽습니다. 실패 종류를 `ShellEnvFailure`로 바꾸고 `static SHELL_PATH`를 없앱니다. Finder로 띄운 빌드에서 `.zshrc`에만 있는 변수가 CLI에 가는지 확인합니다 (FR-AGENT-012, FR-AGENT-016, SC-AGENT-003)
- [x] 2.3 Windows `read_shell_env`: PowerShell(7 우선), Git Bash, 명령 프롬프트(읽지 않음)와 `git_bash_available`을 구현합니다. Windows 대상 clippy를 통과시킵니다 (FR-AGENT-014, FR-START-011)
- [x] 2.4 `ShellEnvState`를 앱 상태로 등록하고, `Detection`에 `shell_env_failure`를 더합니다. `tasks/agent_detection::run`이 셸 환경을 읽고 남은 시간으로 감지하게 바꿉니다. 셸이 끝나지 않을 때 감지가 10초 안에 끝나는 테스트를 가짜 Host로 둡니다 (FR-AGENT-015, FR-AGENT-018, SC-AGENT-001, SC-AGENT-003)
- [x] 2.5 `AgentHost`, `AgentCli`(모델 조회, 로그인 확인)가 `ShellEnv`를 받아 `Command::envs`로 넘기게 바꿉니다. 모델 조회와 로그인 확인이 셸을 띄우지 않는지 확인합니다 (FR-AGENT-012, FR-AGENT-014, FR-AGENT-015, FR-AGENT-011)

### 3. WSL 로그인 셸 실행(Rust IO)

- [x] 3.1 WSL 프로브, 모델 조회, 로그인 확인 명령을 `login_shell_script`로 감싸고 출력을 `after_fence`로 거릅니다 (FR-AGENT-013)
- [x] 3.2 WSL `hook_target`과 `codex_config`가 로그인 셸에서 `HOME`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`을 읽어 `config_dir`로 폴더를 정하게 바꿉니다 (FR-HOOK-002, FR-START-004)

### 4. 훅과 모델의 설정 폴더(Rust)

- [x] 4.1 `InstallTarget`에 `claude_dir`, `codex_dir`를 두고 `install.rs`가 이 경로를 쓰게 바꿉니다. 바뀐 폴더에 설치되고 이전 폴더는 그대로인 테스트를 `ConfigFs` 가짜 구현으로 씁니다 (FR-HOOK-002)
- [x] 4.2 macOS와 Windows `hook_target`, `codex_config`가 `ShellEnvState`의 환경으로 `config_dir`를 계산하게 바꿉니다 (FR-HOOK-002, FR-START-004)

### 5. 명령과 시작 흐름(Rust shell)

- [x] 5.1 `commands/shell.rs`에 `shell_setup`, `set_shell`을 추가하고 `lib.rs`에 등록합니다. 저장한 Git Bash가 없을 때 PowerShell이 선택되는지 확인합니다 (FR-START-011, FR-START-012)
- [x] 5.2 `detect_with_shell`을 추가합니다. 첫 호출은 시작 감지로 publish하고, 같은 셸이면 보관한 결과, 다른 셸이면 `Trigger::Startup`으로 다시 감지하고 훅 설치를 띄웁니다. `platform::DETECT_AT_STARTUP`이 false면 `setup`에서 시작 감지를 띄우지 않습니다 (FR-AGENT-007, FR-START-013, FR-HOOK-002)
- [x] 5.3 `redetect_agents`가 Windows에서 마지막 셸로 감지하게 바꿉니다. 바인딩을 다시 만들고 `pnpm lint:rust`, `pnpm test:rust`를 통과시킵니다 (FR-AGENT-007, FR-AGENT-014)

### 6. 온보딩 core(TS)

- [x] 6.1 `stepper.ts`에 `shell` 단계와 `stepsFor`, 단계 목록을 받는 `nextStep`/`prevStep`, 셸 단계의 이전/다음 조건을 넣습니다. test-writer가 macOS 4단계와 Windows 5단계 시나리오로 테스트를 먼저 씁니다 (FR-START-002, FR-START-005)
- [x] 6.2 `domain/shell.ts`의 `shellLabel`, `shellDescription`, `shouldDetect` 테스트를 쓰고 구현합니다 (FR-START-011, FR-START-013)
- [x] 6.3 `detectionView.ts`에 `shellEnvNotice` 테스트를 쓰고 구현합니다. 실패 이유 네 가지와 실패 없음 경우를 확인합니다 (FR-AGENT-018)

### 7. 온보딩 화면 연결(TS IO, React)

- [x] 7.1 `api/index.ts`에 `getShellSetup`, `setShell`, `detectWithShell`을 추가합니다. `pnpm typecheck`로 바인딩과 맞는지 확인합니다 (FR-START-011, FR-START-012, FR-START-013)
- [x] 7.2 `DetectionStep.tsx`에 셸 환경 실패 안내를 보여 줍니다. `.zshrc`에 `sleep 10`을 넣은 macOS에서 안내가 보이고, 고친 뒤 다시 감지하면 사라지는지 확인합니다 (FR-AGENT-018)
- [x] 7.3 `ShellStep.tsx`를 만들고 `Onboarding.tsx`에 연결합니다. Windows에서만 셸 단계를 보이고, 마운트 때 감지를 부르지 않으며, 다음을 누를 때 `shouldDetect`면 `detectWithShell`을 부르게 합니다. 고르면 `setShell`로 바로 저장합니다. macOS에서 단계가 4개인지 확인합니다 (FR-START-002, FR-START-005, FR-START-011, FR-START-012, FR-START-013, FR-AGENT-007)

### 8. 검증

- [x] 8.1 `pnpm format:check`, `pnpm lint`, `pnpm test`, `pnpm build`, `pnpm format:check:rust`, `pnpm lint:rust`, `pnpm test:rust`가 모두 통과합니다 (SC-AGENT-001, SC-AGENT-003)
- [x] 8.2 앱 데이터 폴더 파일, renderer가 받은 명령 결과, 로그에 셸의 API 키 값이 없는지 확인합니다 (FR-AGENT-017, FR-AGENT-011)

### 남은 확인 (Windows 기기)

PR을 머지하기 전에 Windows 기기에서 확인합니다.

- 2.3: PowerShell 프로필에만 적은 변수가 CLI에 넘어가는지. Git Bash `.bashrc`에 `/c/...`처럼 적은 `CLAUDE_CONFIG_DIR`이 Windows 경로로 넘어오는지
- 3.1: `.bashrc`가 문구를 찍는 WSL 배포판에서 감지, 모델 목록, 로그인 확인이 되는지. 로그인 셸이 느려도 시간 상한 안에 끝나는지
- 7.3: 처음 실행하면 셸 선택 단계가 맨 앞에 나오는지, 셸을 바꾸고 다음을 누르면 다시 감지하는지, 같은 셸이면 다시 감지하지 않는지
