# Plan

spec-hash: ba7ba9aa64175857

## 설계

### 층 배치

| 층 | 파일 | 하는 일 |
| --- | --- | --- |
| core(Rust) | `src-tauri/src/hook/domain/model.rs` | 설치 결과, 훅 이벤트, 설치 대상 타입 |
| core(Rust) | `src-tauri/src/hook/domain/command.rs` | 에이전트·버전별 이벤트 목록, OS별 훅 명령 문자열, 앱 항목 식별 |
| core(Rust) | `src-tauri/src/hook/domain/claude.rs` | `settings.json` 병합 |
| core(Rust) | `src-tauri/src/hook/domain/codex.rs` | `hooks.json` 병합, 신뢰 해시 계산, `config.toml` 병합 |
| core(Rust) | `src-tauri/src/hook/domain/request.rs` | 훅 요청 판정(토큰, 경로)과 본문 해석, 큰 본문 요약 |
| core(Rust) | `src-tauri/src/hook/domain/endpoint.rs` | 엔드포인트 파일 내용, 토큰 비교 |
| IO(Rust) | `src-tauri/src/hook/fs.rs` | 설정 파일을 읽고 쓰는 `ConfigFs` trait, 원자적 쓰기와 쓰기 권한 확인을 하는 기본 구현 |
| IO(Rust) | `src-tauri/src/hook/install.rs` | `ConfigFs`로 파일을 읽어 core로 새 내용을 만들고 쓰는 설치 흐름 |
| IO(Rust) | `src-tauri/src/hook/server.rs` | `127.0.0.1` HTTP 서버. 본문을 읽어 core에 넘기고 이벤트를 콜백으로 낸다 |
| 스크립트 | `src-tauri/resources/hooks/namaste-hook.sh`, `namaste-hook.cmd` | 훅이 실행하는 스크립트. `include_str!`로 바이너리에 넣는다 |
| IO(Rust) | `src-tauri/src/platform/macos.rs`, `windows.rs`, `mod.rs` | 환경별 `ConfigFs`(macOS 홈, Windows 홈, WSL 배포판의 `\\wsl$` 경로와 `chmod`), WSL의 홈과 Windows 경로 변환 |
| 공유 상태(Rust) | `src-tauri/src/state.rs` | 설치 결과 보관(`watch`), 서버 토큰 |
| 이벤트(Rust) | `src-tauri/src/events.rs` | 이벤트 이름 상수 `HOOK_EVENT`, `HOOK_INSTALLATION` |
| 백그라운드 작업 | `src-tauri/src/tasks/hook_server.rs`, `tasks/hook_install.rs` | 서버를 띄우고 엔드포인트 파일을 씀, 감지 결과를 기다려 설치 |
| 명령 | `src-tauri/src/commands/hook.rs`, `commands/agent.rs`, `lib.rs` | `hook_installation` 명령, 다시 감지 뒤 설치, single-instance 플러그인 등록 |
| IO(TS) | `src/renderer/src/api/index.ts`, `types/ipc.ts` | 명령과 이벤트 구독, 이벤트 이름 상수 |
| core(TS) | `src/renderer/src/features/hook-events/hookEventsView.ts` | 이벤트 목록 갱신(200개), 화면 줄 변환 |
| React | `src/renderer/src/features/hook-events/HookEventsPanel.tsx` | 설치 결과와 이벤트 목록 |
| 화면 조립 | `src/renderer/src/windows/main/App.tsx` | 패널 추가 |

- `hook/`은 IO가 섞인 로직 모듈이라 순수한 부분을 `hook/domain/`에 둡니다. `hook/`은 `tauri`를 import하지 않습니다. 서버는 이벤트를 콜백으로 넘기고, `tasks/hook_server.rs`가 그 콜백에서 `emit`합니다.
- `#[cfg(target_os)]` 코드는 `platform/`에만 둡니다. `hook/install.rs`는 `ConfigFs` trait만 압니다.
- TS core 파일은 하나라서 `domain/`을 만들지 않습니다. 이 화면에서만 쓰는 상태라 store를 만들지 않습니다.

### 타입과 경계

ts-rs로 내보내는 타입:

```rust
/// 설치 결과. 에이전트와 실행 환경마다 하나.
#[serde(tag = "status", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum InstallStatus { Installed, AlreadyInstalled, Failed { reason: InstallFailure } }

#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum InstallFailure {
    InvalidConfig { file: String },   // JSON/TOML로 해석할 수 없거나 모양이 다름(FR-HOOK-004)
    NotWritable { file: String },     // 쓰기 권한 없음(FR-HOOK-006)
    CodexHooksDisabled { file: String }, // 사용자가 [features] hooks = false로 둠(clarify Q2)
    Io { message: String },           // 그 밖의 파일·프로세스 실패
}

pub struct InstallEntry { pub agent: AgentKind, pub environment: Environment, pub status: InstallStatus }
pub struct HookInstallation { pub entries: Vec<InstallEntry> }

/// renderer에 넘기는 훅 이벤트(FR-HOOK-009, FR-HOOK-013).
pub struct HookEvent {
    pub agent: AgentKind,
    pub event: String,               // 본문의 hook_event_name, 없으면 URL의 이벤트 이름
    pub received_at: u64,            // Unix ms
    #[ts(optional)] pub session_id: Option<String>,
    #[ts(optional)] pub tool_name: Option<String>,
    #[ts(optional)] pub tool_use_id: Option<String>,
    #[ts(optional)] pub is_error: Option<bool>,
    #[ts(optional)] pub cwd: Option<String>,
    #[ts(optional)] pub payload: Option<serde_json::Value>, // 1MB 이하면 원래 입력 전체
    pub truncated: bool,             // 1MB를 넘어 payload를 뺐다
}
```

- `AgentKind`, `Environment`는 `agent/domain/model.rs`의 것을 그대로 씁니다.
- 명령: `hook_installation() -> Result<HookInstallation, ()>`. 첫 설치가 끝날 때까지 기다렸다가 보관한 결과를 줍니다.
- 이벤트(`events.rs`와 `types/ipc.ts`에 같은 이름): `hook-event`(payload `HookEvent`), `hook-installation`(payload `HookInstallation`, 설치가 끝날 때마다).
- capability: `listen`은 `core:default`에 든 `core:event:default`로 됩니다. 새 권한은 없습니다. single-instance 플러그인은 renderer 권한이 필요 없습니다.
- HTTP: `POST http://127.0.0.1:<port>/hook/<agent>/<Event>`, 헤더 `X-Namaste-Token`. 토큰이 틀리면 401, 경로가 틀리면 404, 그 밖에는 본문을 해석하지 못해도 204를 돌려줍니다(훅은 응답을 보지 않음).
- 앱 파일(사용자 홈 아래, 공백 없는 경로를 위해 앱 데이터 폴더 대신 씀): `~/.namaste-world/endpoint`(포트와 토큰, 0600), `~/.namaste-world/hooks/namaste-hook.sh`·`namaste-hook.cmd`(0700 폴더). WSL 배포판에는 그 배포판 홈의 `~/.namaste-world/hooks/namaste-hook.sh`를 두고, 엔드포인트는 Windows 쪽 파일을 `/mnt/<드라이브>/...` 경로로 읽습니다.
- 의존성(`src-tauri/Cargo.toml`):
  - `axum = "0.8"`: HTTP 서버. tokio 위에서 돌고 경로와 헤더 처리를 맡깁니다
  - `serde_json = { version = "1", features = ["preserve_order"] }`: 설정 파일의 키 순서를 지킵니다(FR-HOOK-004)
  - `toml_edit = "0.25"`: `config.toml`의 주석과 서식을 지키며 고칩니다. 이미 의존성 그래프에 있습니다
  - `sha2 = "0.10"`: Codex 신뢰 해시
  - `getrandom = "0.4"`: 토큰. 이미 의존성 그래프에 있습니다
  - `tokio-util = { version = "0.7", features = ["io"] }`: 큰 본문을 메모리에 다 올리지 않고 스트림으로 해석
  - `tauri-plugin-single-instance = "2"`: FR-HOOK-014
  - `ts-rs`에 `serde-json-impl` 기능 추가(`payload` 타입)

### 시그니처

core(Rust, `hook/domain/`):

```rust
/// 설치할 곳. home은 그 환경 안의 경로 표기(macOS·WSL은 POSIX, Windows는 `C:\...`)다.
pub struct InstallTarget {
    pub agent: AgentKind,
    pub environment: Environment,
    pub version: String,            // 감지 결과의 버전
    pub home: String,
    pub endpoint_path: String,      // 그 환경에서 읽을 엔드포인트 파일 경로
}
pub enum ShellFamily { Posix, Windows }   // Macos·Wsl → Posix, Windows → Windows

/// 설치할지. 감지 상태가 Installed인 항목만 true(FR-HOOK-002).
pub fn should_install(entry: &AgentEntry) -> bool;

/// 그 에이전트·버전이 제공하는 훅 이벤트(FR-HOOK-003).
/// Claude: 31개(SessionStart, Setup, UserPromptSubmit, UserPromptExpansion, PreToolUse, PermissionRequest,
///   PermissionDenied, PostToolUse, PostToolUseFailure, PostToolBatch, Notification, MessageDisplay, SubagentStart,
///   SubagentStop, TaskCreated, TaskCompleted, Stop, StopFailure, TeammateIdle, InstructionsLoaded, ConfigChange,
///   CwdChanged, DirectoryAdded, FileChanged, PreCompact, PostCompact, PreModelSwitch, PostModelSwitch,
///   Elicitation, ElicitationResult, SessionEnd). 등록만 해도 git worktree 기본 동작을 대신하는
///   WorktreeCreate, WorktreeRemove는 넣지 않는다.
/// Codex: 0.169.0 미만은 11개(PreToolUse, PermissionRequest, PostToolUse, PreCompact, PostCompact, SessionStart,
///   SessionEnd, UserPromptSubmit, SubagentStart, SubagentStop, Stop), 0.169.0 이상은 PreModelRequest,
///   PostModelRequest, Interrupt를 더한 14개. 버전을 해석하지 못하면 11개.
pub fn hook_events(agent: AgentKind, version: &str) -> Vec<&'static str>;

/// 앱 훅 항목에 들어가는 표시. 이 문자열이 command나 args에 있으면 앱 항목이다.
pub const HOOK_MARKER: &str = "namaste-world-hook";

/// 이벤트 하나에 등록할 훅 handler(JSON 객체). matcher는 넣지 않는다(모든 도구에 실행).
/// Posix: {"type":"command","command":"<sh 한 줄>","timeout":t}
///   sh 한 줄: `: namaste-world-hook; f="<home>/.namaste-world/hooks/namaste-hook.sh"; if [ -f "$f" ]; then /bin/sh "$f" <agent> <Event> "<endpoint>" >/dev/null 2>&1; else cat >/dev/null 2>&1; fi; exit 0`
/// Windows + Codex: {"type":"command","command":"<cmd 한 줄>","timeout":t}  (Codex는 `cmd.exe /C`로 실행)
///   cmd 한 줄: `if exist "<script>" ("<script>" <agent> <Event> namaste-world-hook) else (more >nul)`
/// Windows + Claude: {"type":"command","command":"cmd.exe","args":["/d","/c","<cmd 한 줄>"],"timeout":t}
///   (Git Bash 유무와 상관없이 셸을 거치지 않는 exec 형식)
/// t는 SessionEnd만 3, 나머지는 5(초).
pub fn hook_handler(target: &InstallTarget, shell: ShellFamily, event: &str) -> serde_json::Value;

/// 설정 파일 병합 결과.
pub enum Merge { Unchanged, Updated(String) }
pub enum MergeError { InvalidJson, InvalidToml, NotObject, HooksDisabled }

/// settings.json 병합(FR-HOOK-002, 003, 004). existing이 None이면 빈 객체에서 시작한다.
/// 앱 항목은 HOOK_MARKER가 든 handler만 가진 group이다. desired의 이벤트마다:
///   앱 group이 이미 있으면 첫 앱 group 자리에서 {"hooks":[handler]}로 바꾸고 나머지 앱 group은 지운다.
///   없으면 {"hooks":[handler]}를 맨 끝에 붙인다.
/// desired에 없는 이벤트의 앱 group은 지우고, 그래서 빈 배열이 되면 이벤트 키도 지운다(TQ1).
/// 앱이 아닌 group과 키, 순서는 그대로다. 앱 group을 옮기지 않으므로 다른 도구 group의 위치도 바뀌지 않는다.
/// 결과가 기존과 의미상 같으면 Unchanged.
/// 해석할 수 없으면 InvalidJson, 최상위나 hooks가 객체가 아니면 NotObject.
pub fn merge_claude_settings(existing: Option<&str>, desired: &[(&str, serde_json::Value)]) -> Result<Merge, MergeError>;

/// hooks.json 병합. 규칙은 merge_claude_settings와 같고, 앱 group이 놓인 위치도 돌려준다.
pub fn merge_codex_hooks(existing: Option<&str>, desired: &[(&str, serde_json::Value)])
    -> Result<(Merge, Vec<CodexHookSlot>), MergeError>;
pub struct CodexHookSlot { pub event: String, pub group_index: usize, pub handler: serde_json::Value }

/// Codex 신뢰 해시. {event_name: snake_case 이벤트, hooks: [{type, command, timeout, async}]}를
/// 키 정렬한 compact JSON으로 만들어 SHA-256, "sha256:<소문자 hex>". matcher는 넣지 않는다.
/// 예: PreToolUse, command "echo hi", timeout 600 → 매처가 없을 때의 해시(테스트는 Codex 소스 계산과 맞춘 값을 씀).
pub fn codex_hook_hash(event: &str, handler: &serde_json::Value) -> String;

/// config.toml 병합(FR-HOOK-006, clarify Q2).
/// [features] hooks(또는 legacy codex_hooks)가 false면 Err(HooksDisabled). 없으면 hooks = true를 넣는다.
/// 각 slot에 [hooks.state."<hooks_json_path>:<snake>:<group_index>:0"] trusted_hash를 넣는다(enabled는 건드리지 않음).
/// 다른 키, 주석, 서식은 그대로다. 해석할 수 없으면 InvalidToml.
pub fn merge_codex_config(existing: Option<&str>, hooks_json_path: &str, slots: &[CodexHookSlot]) -> Result<Merge, MergeError>;

/// 엔드포인트 파일 내용. "NAMASTE_PORT=<port>\nNAMASTE_TOKEN=<token>\n".
pub fn endpoint_file(port: u16, token: &str) -> String;
/// 길이가 같을 때 끝까지 비교하는 토큰 비교.
pub fn token_matches(expected: &str, given: Option<&str>) -> bool;

/// 요청 판정. 토큰이 틀리면 Unauthorized, 경로의 agent가 claude/codex가 아니면 NotFound.
pub enum Rejection { Unauthorized, NotFound }
pub fn route(agent_segment: &str, expected_token: &str, given_token: Option<&str>) -> Result<AgentKind, Rejection>;

/// 1MB 이하 본문을 해석한다. JSON 객체가 아니면 None(FR-HOOK-009 "해석할 수 없는 입력").
/// 요약 필드는 session_id, tool_name, tool_use_id, cwd, hook_event_name, 그리고 is_error
/// (hook_event_name이 PostToolUseFailure면 true, tool_response가 객체이고 is_error나 success가 bool이면 그 값, 그 밖에는 없음).
pub fn parse_full(agent: AgentKind, url_event: &str, body: &[u8], received_at: u64) -> Option<HookEvent>;

/// 1MB를 넘는 본문을 읽기 도구로 스트림 해석한다. 요약 필드만 담고 payload: None, truncated: true.
/// JSON 객체가 아니면 None.
pub fn parse_summary<R: std::io::Read>(agent: AgentKind, url_event: &str, body: R, received_at: u64) -> Option<HookEvent>;

pub const FULL_BODY_LIMIT: usize = 1024 * 1024;
```

IO(Rust):

```rust
/// 한 실행 환경의 파일 접근. path는 그 환경 안의 경로 표기다.
pub trait ConfigFs: Sync {
    /// 없으면 Ok(None).
    fn read(&self, path: &str) -> impl Future<Output = io::Result<Option<String>>> + Send;
    /// 부모 폴더를 만들고, 기존 파일이 있으면 먼저 쓰기 권한을 확인한 뒤(없으면 PermissionDenied) 원자적으로 바꾼다.
    /// 심볼릭 링크면 링크가 가리키는 파일을 바꾼다. mode는 POSIX 권한(Windows 파일에는 무시).
    fn write(&self, path: &str, contents: &str, mode: u32) -> impl Future<Output = io::Result<()>> + Send;
}

/// 한 대상에 훅을 설치한다. 설정 파일을 먼저 모두 읽어 병합할 수 있는지 확인한 뒤(TQ3, TQ5),
/// 스크립트 → Claude settings.json, 또는 스크립트 → Codex config.toml → hooks.json 순서로 바뀐 파일만 쓴다.
/// Codex는 신뢰 정보(config.toml)를 먼저 써서, hooks.json 쓰기 전에 끊겨도 신뢰 없는 앱 훅이 남지 않게 한다.
/// 바뀐 파일이 없으면 AlreadyInstalled, 하나라도 쓰면 Installed, 실패하면 Failed(이유).
/// 새 설정 파일은 0o644, 스크립트는 0o700으로 쓰고, 기존 파일은 원래 권한을 지킨다(TQ4).
pub async fn install<F: ConfigFs>(fs: &F, target: &InstallTarget, shell: ShellFamily) -> InstallStatus;

/// 서버. 127.0.0.1의 빈 포트에 묶고, 받은 이벤트를 on_event로 넘긴다.
pub async fn bind() -> io::Result<(tokio::net::TcpListener, u16)>;
pub async fn serve(listener: TcpListener, token: String, on_event: impl Fn(HookEvent) + Send + Sync + 'static);

/// state.rs. 설치 결과 보관. AgentDetection과 같은 모양.
pub struct HookInstallState { /* watch::Sender<Option<HookInstallation>> */ }
impl HookInstallState { pub fn new() -> Self; pub async fn latest(&self) -> Option<HookInstallation>; pub fn publish(&self, value: HookInstallation); }

#[tauri::command] pub async fn hook_installation(state: State<'_, HookInstallState>) -> Result<HookInstallation, ()>;
```

core(TS, `features/hook-events/hookEventsView.ts`):

```ts
export const EVENT_LIMIT = 200;
/** 새 이벤트를 끝에 붙이고 EVENT_LIMIT개를 넘으면 앞(오래된 것)부터 뺀다. 입력 배열은 바꾸지 않는다. */
export function appendEvent(list: readonly HookEvent[], event: HookEvent, limit?: number): readonly HookEvent[];
/** 이벤트 한 줄. time은 'HH:MM:SS'(로컬 시각), agentLabel은 'Claude Code' | 'Codex'. */
export type EventRow = { readonly key: string; readonly time: string; readonly agentLabel: string; readonly event: string };
export function toEventRow(event: HookEvent, index: number): EventRow;
/** 설치 결과 한 줄. statusLabel: '새로 설치함' | '이미 설치됨' | '실패', detail은 실패 이유 문구. */
export type InstallRow = { readonly key: string; readonly agentLabel: string; readonly environmentLabel: string; readonly statusLabel: string; readonly detail?: string };
export function toInstallRows(installation: HookInstallation): readonly InstallRow[];
```

api(TS): `getHookInstallation(): ResultAsync<HookInstallation, IpcError>`, `onHookEvent(cb): ResultAsync<UnlistenFn, IpcError>`, `onHookInstallation(cb): ResultAsync<UnlistenFn, IpcError>`.

### 결정

- **Orca 방식을 따릅니다.**
  - 설정에는 포트와 토큰을 적지 않고, 훅 스크립트가 실행될 때마다 엔드포인트 파일을 읽습니다. 앱을 다시 켜서 포트가 바뀌어도 설정 파일을 고치지 않습니다(FR-HOOK-008).
  - 엔드포인트 파일은 셸로 실행(`.`)하지 않고 `KEY=VALUE` 줄로 읽습니다. 파일이 바뀌어도 코드가 실행되지 않게 하기 위해서입니다.
  - 훅 명령은 스크립트가 있는지 먼저 보고, 없으면 입력만 읽고 끝냅니다(FR-HOOK-005 "스크립트가 지워짐").
  - `curl`에 `--connect-timeout 0.5 --max-time 1.3 --noproxy 127.0.0.1`을 주고, 결과와 상관없이 `exit 0`, 표준 출력 없음으로 끝냅니다. 두 에이전트 모두 출력이 없고 종료 코드가 0이면 동작이 바뀌지 않습니다(SC-HOOK-001, FR-HOOK-005).
  - Orca가 쓰는 spool(못 보낸 이벤트 파일)은 쓰지 않습니다(clarify Q1, FR-HOOK-012).
- **훅 이벤트 이름과 에이전트를 URL에 넣습니다.** 본문이 아무리 커도, 해석하지 못해도 어느 에이전트의 어떤 이벤트인지는 압니다.
- **WSL에서는 `curl.exe`(Windows 쪽)로 먼저 보냅니다.** WSL2 기본 네트워크(NAT)에서는 배포판의 `127.0.0.1`이 Windows에 닿지 않습니다. Windows interop로 `curl.exe`를 부르면 Windows의 `127.0.0.1`로 갑니다. `curl.exe`가 없으면 배포판의 `curl`로 `127.0.0.1`에 보냅니다(mirrored 모드에서 동작). `curl.exe`를 미리 실행해 보지 않고, interop가 멈춰도 2초 안에 끝나도록 coreutils `timeout`으로 한 번 더 감쌉니다. Orca도 같은 두 경로를 씁니다. 배포판에 `curl`을 따로 설치하지 않아도 됩니다(FR-HOOK-011).
- **Windows 훅 명령**
  - Codex는 `%COMSPEC% /C "<command>"`로 훅을 실행하므로 `cmd` 한 줄로 `.cmd` 스크립트를 부릅니다. Windows 10 1803부터 들어 있는 `curl.exe`로 보냅니다.
  - Claude Code는 Git Bash가 있으면 bash, 없으면 PowerShell로 훅을 실행합니다. 둘 다에서 같게 동작하도록 `args`가 있는 exec 형식으로 `cmd.exe`를 직접 실행합니다. PowerShell은 시작에 1초 가까이 걸려 SC-HOOK-001을 넘길 수 있어 쓰지 않습니다.
- **WSL 설치는 Windows에서 `\\wsl$\<배포판>\...` 경로로 파일을 읽고 씁니다.** 병합 로직을 배포판 안에 다시 만들지 않기 위해서입니다. 배포판 홈은 `wsl.exe -d <배포판> -e sh -c 'printf %s "$HOME"'`로, 엔드포인트 경로는 `wslpath -u`로 얻고, 실행 권한은 `wsl.exe -e chmod`로 줍니다.
- **Codex 신뢰는 앱이 기록합니다(clarify Q2).** 신뢰 해시가 없으면 Codex가 훅을 조용히 건너뛰기 때문입니다. 해시 계산은 Codex 0.146.0 소스(`hooks/src/engine/discovery.rs`의 `command_hook_hash`)와 같고, 이 Mac의 `~/.codex/config.toml`에 이미 있는 신뢰 해시 8개를 같은 방법으로 다시 계산해 모두 맞는 것을 확인했습니다. 상태 키의 group 번호는 앱 group의 실제 위치입니다. 다른 도구가 앱 group 앞에 항목을 넣으면 다음 실행에서 새 위치로 다시 기록합니다.
- **`[features] hooks`는 Codex 0.146에서 기본으로 켜져 있지만, 없으면 `hooks = true`를 적습니다.** 더 낮은 버전에서도 훅이 돌게 하기 위해서이고 적어도 해가 없습니다.
- **이미 설치됨 판정은 쓰기 직전 결과 비교로 합니다.** 스크립트, 설정 파일 각각 새 내용이 지금 내용과 같으면 쓰지 않습니다. 모두 같으면 AlreadyInstalled입니다(FR-HOOK-002 "이미 설치됨").
- **쓰기 권한은 바꾸지 않습니다(clarify Q3).** 기존 파일은 쓰기 모드로 열어 권한을 먼저 확인하고, 실패하면 NotWritable로 남깁니다. 쓸 때는 임시 파일에 쓰고 바꿔치기해 중간에 끊겨도 설정 파일이 깨지지 않게 합니다.
- **큰 본문은 1MB까지 메모리에 읽고, 넘으면 남은 부분을 스트림으로 해석합니다.** 크기와 상관없이 요약 필드를 얻으면서 메모리는 1MB 근처로 묶입니다(FR-HOOK-013).
- **설치는 감지 결과가 나온 뒤 합니다.** `tasks/hook_install.rs`가 `AgentDetection::latest()`를 기다린 뒤 설치하고, `redetect_agents`도 새 결과를 publish한 뒤 같은 설치를 실행합니다(clarify Q4). 설치 대상끼리는 동시에 실행해 한 대상의 실패나 지연이 다른 대상을 막지 않습니다(FR-HOOK-007).
- **서버는 설치보다 먼저 뜹니다.** 엔드포인트 파일을 쓴 뒤 설치하므로, 설치 직후의 훅도 바로 앱에 닿습니다. 서버를 열지 못해도 설치는 하고 결과를 보관해 화면이 기다리며 멈추지 않게 합니다. 앱이 끝날 때 엔드포인트 파일을 지워, 꺼진 뒤 그 포트를 차지한 다른 프로세스로 훅 본문이 가지 않게 합니다.
- **설치는 한 번에 하나만 돕니다.** 시작 설치 중에 다시 감지가 와도 같은 설정 파일을 동시에 고치지 않도록 `tasks/hook_install.rs`에서 `Mutex`로 줄 세웁니다.
- **앱 훅 group은 자리를 지킵니다.** Codex 신뢰 상태 키는 group 위치로 정해져, 앱 group을 매번 맨 끝으로 옮기면 뒤에 있던 다른 도구(Orca 등) group이 앞으로 당겨져 그 도구의 신뢰가 풀립니다. 그래서 이미 있는 앱 group은 그 자리에서 바꾸고, 없을 때만 끝에 붙입니다.
- **WSL 파일은 배포판 안에서 링크와 권한을 다룹니다.** Windows에서는 배포판 안의 심볼릭 링크를 따라가지 못하고 POSIX 권한도 줄 수 없어, `readlink -f`로 실제 파일을 찾고 `stat`으로 기존 권한을 읽은 뒤 쓰고 나서 `chmod`로 맞춥니다.
- **화면은 원래 입력을 쌓지 않습니다.** 이 화면은 이벤트 이름만 보여 주므로 최대 1MB인 `payload`를 빼고 200개를 쌓습니다.
- **single instance는 `tauri-plugin-single-instance`로 합니다.** 두 번째 실행이 오면 첫 앱이 `main` 창을 보이고 앞으로 가져옵니다(FR-HOOK-014). 이 플러그인은 빌더에 가장 먼저 등록해야 합니다.
- **디스크에 이벤트를 남기지 않습니다(FR-HOOK-012).** 서버와 작업은 이벤트 내용을 로그로 찍지 않습니다. 실패 로그에도 본문을 넣지 않습니다.
- 버린 방안: 앱 데이터 폴더(`~/Library/Application Support/...`)는 경로에 공백이 있어 셸 명령 따옴표가 복잡해져 쓰지 않습니다. 고정 포트는 다른 프로그램과 겹칠 수 있어 쓰지 않습니다. HTTP 서버를 직접 짜는 방안은 요청 파싱 버그 위험이 커서 `axum`을 씁니다.

### 위험

- [위험] 오래된 Claude Code가 모르는 훅 이벤트 이름을 설정 오류로 볼 수 있음 → 감지된 버전(이 Mac 2.1.325)에서 31개 목록으로 동작을 확인합니다. 문제가 보이면 `hook_events`에 버전 구간을 더합니다.
- [위험] Codex 0.146 미만에서 이벤트나 신뢰 방식이 다름 → 0.146에서 확인한 11개만 등록합니다. 그보다 낮은 버전은 동작을 보장하지 않습니다.
- [위험] Windows의 `cmd` 따옴표 처리(Codex의 `/C "<command>"`, Claude exec 형식의 `cmd.exe /d /c`)가 예상과 달라 스크립트가 실행되지 않음 → Windows에서 공백·한글 경로로 직접 확인합니다(작업 9.2). 맞지 않으면 따옴표 규칙만 `hook_handler`에서 고칩니다.
- [위험] Claude Code의 `args`(exec 형식)를 모르는 버전이면 `cmd.exe`가 표준 입력을 명령으로 읽음 → 실패해도 비차단 오류라 에이전트 동작은 그대로입니다. 확인한 최신 버전 기준으로 둡니다.
- [위험] SessionEnd는 Claude Code가 1.5초, Codex가 기본 1초만 기다림 → 그 안에 못 보내면 그 이벤트만 빠지고 에이전트는 그대로 끝납니다.
- [위험] `\\wsl$` 경로는 배포판이 켜져 있어야 열림 → 설치는 감지에서 설치됨으로 나온 배포판에만 하므로 그 시점에 켜져 있습니다.
- [위험] CI는 Linux에서 돌아 `platform/macos.rs`, `windows.rs`가 컴파일되지 않음 → 병합, 해시, 명령 문자열, 요청 해석은 모두 `hook/domain/`에 두어 CI 테스트로 확인합니다. `install`은 메모리 `ConfigFs`로 테스트합니다.
- [위험] `serde_json`의 `preserve_order`는 크레이트 전체에 적용됨 → 기존 감지 코드는 JSON 키 순서에 기대지 않아 영향이 없습니다.

## 작업

### 1. 타입과 의존성

- [x] 1.1 의존성을 추가하고 `hook/domain/model.rs`에 `InstallStatus`, `InstallFailure`, `InstallEntry`, `HookInstallation`, `HookEvent`를 ts-rs derive와 함께 정의합니다. 확인: `pnpm test:rust`가 `types/bindings/`에 TS 타입을 만듦 (FR-HOOK-007, FR-HOOK-009)

### 2. 훅 명령과 설정 병합 (core)

- [x] 2.1 `should_install`, `hook_events`, `hook_handler`의 시그니처를 만들고 `test-writer`가 테스트를 씁니다. 확인: 새 테스트가 실패함 (FR-HOOK-002, FR-HOOK-003, FR-HOOK-005, FR-HOOK-011)
- [x] 2.2 2.1의 함수를 구현합니다. 확인: 2.1 테스트 통과 (FR-HOOK-002, FR-HOOK-003, FR-HOOK-005, FR-HOOK-011)
- [x] 2.3 `merge_claude_settings`의 시그니처를 만들고 `test-writer`가 테스트를 씁니다(앱 group 자리 유지, 다른 훅과 순서 보존, 훅 외 설정 보존, 이미 설치됨, 낡은 항목 교체, 중복 없음, 손상된 파일). 확인: 새 테스트가 실패함 (FR-HOOK-002, FR-HOOK-004)
- [x] 2.4 `merge_claude_settings`를 구현합니다. 확인: 2.3 테스트 통과 (FR-HOOK-002, FR-HOOK-004)
- [x] 2.5 `merge_codex_hooks`, `codex_hook_hash`, `merge_codex_config`의 시그니처를 만들고 `test-writer`가 테스트를 씁니다(해시 고정값, slot 위치, `hooks = false`, 주석 보존). 확인: 새 테스트가 실패함 (FR-HOOK-004, FR-HOOK-006)
- [x] 2.6 2.5의 함수를 구현합니다. 확인: 2.5 테스트 통과 (FR-HOOK-004, FR-HOOK-006)

### 3. 요청 해석 (core)

- [x] 3.1 `endpoint_file`, `token_matches`, `route`, `parse_full`, `parse_summary`의 시그니처를 만들고 `test-writer`가 테스트를 씁니다(토큰 틀림, 알 수 없는 에이전트, JSON이 아닌 본문, 1MB 경계, 3MB 본문의 요약 필드, is_error 판정). 확인: 새 테스트가 실패함 (FR-HOOK-001, FR-HOOK-009, FR-HOOK-013)
- [x] 3.2 3.1의 함수를 구현합니다. 확인: 3.1 테스트 통과 (FR-HOOK-001, FR-HOOK-009, FR-HOOK-013)

### 4. 훅 스크립트

- [x] 4.1 `namaste-hook.sh`를 씁니다(엔드포인트 줄 읽기, WSL이면 `curl.exe` 먼저, 시간 제한, 출력 없음, `exit 0`). 확인: macOS에서 서버 없이 실행하면 2초 안에 0으로 끝나고 출력이 없음, 응답하지 않는 서버(`nc -l`)에도 2초 안에 끝남 (FR-HOOK-005, FR-HOOK-008, SC-HOOK-001)
- [x] 4.2 `namaste-hook.cmd`를 씁니다(엔드포인트 줄 읽기, `curl.exe`, 출력 없음, `exit /b 0`). 확인: Windows에서 서버 없이 실행하면 2초 안에 0으로 끝나고 출력이 없음 (FR-HOOK-005, FR-HOOK-011, SC-HOOK-001)

### 5. 설치

- [x] 5.1 `hook/fs.rs`의 `ConfigFs`와 `hook/install.rs`의 `install` 시그니처를 만들고, `test-writer`가 메모리 `ConfigFs`로 테스트를 씁니다(처음 설치, 다시 실행하면 AlreadyInstalled이고 쓰기 없음, 설정 파일 없음, 쓰기 거부 → NotWritable이고 다른 파일도 안 씀, Codex `hooks = false` → hooks.json도 안 씀, 스크립트 권한 0700). 확인: 새 테스트가 실패함 (FR-HOOK-002, FR-HOOK-004, FR-HOOK-006, FR-HOOK-007)
- [x] 5.2 `install`과 기본 `ConfigFs`(원자적 쓰기, 쓰기 권한 확인, 심볼릭 링크 따라가기)를 구현합니다. 확인: 5.1 테스트 통과 (FR-HOOK-002, FR-HOOK-004, FR-HOOK-006, FR-HOOK-007)
- [x] 5.3 `platform/macos.rs`에 macOS 홈의 설치 대상과 `ConfigFs`를 둡니다. 확인: macOS에서 `pnpm lint:rust` 통과 (FR-HOOK-002, FR-HOOK-011)
- [x] 5.4 `platform/windows.rs`에 Windows 홈과 WSL 배포판(`\\wsl$`, 홈 경로, `wslpath`, `chmod`)의 설치 대상과 `ConfigFs`를 둡니다. 확인: `cargo check --target x86_64-pc-windows-msvc`가 되면 통과, 아니면 사용자가 Windows에서 확인 (FR-HOOK-002, FR-HOOK-008, FR-HOOK-011)

### 6. 서버

- [x] 6.1 `hook/server.rs`의 `bind`, `serve`를 만들고, `test-writer`가 실제 소켓으로 테스트를 씁니다(127.0.0.1에만 묶임, 토큰 틀림 401과 이벤트 없음, 맞으면 이벤트 하나, JSON 아님은 무시, 3MB 본문은 truncated 이벤트). 확인: 테스트 통과 (FR-HOOK-001, FR-HOOK-009, FR-HOOK-013)

### 7. 상태, 작업, 명령

- [x] 7.1 `state.rs`에 `HookInstallState`를 두고 `test-writer`가 `AgentDetection`과 같은 기준의 테스트를 씁니다. 확인: 테스트 통과 (FR-HOOK-007)
- [x] 7.2 `events.rs`, `tasks/hook_server.rs`(토큰 생성, 서버, 엔드포인트 파일, `emit`), `tasks/hook_install.rs`(감지 결과를 기다려 설치하고 publish와 `emit`), `commands/hook.rs`, `redetect_agents` 뒤 설치, single-instance 플러그인을 `lib.rs`에 등록합니다. 확인: `pnpm test:rust`, `pnpm lint:rust` 통과, `pnpm dev`에서 `~/.namaste-world/endpoint`가 생기고 두 번째 실행이 첫 창을 앞으로 가져옴 (FR-HOOK-001, FR-HOOK-002, FR-HOOK-008, FR-HOOK-012, FR-HOOK-014)
- [x] 7.3 `types/ipc.ts`에 이벤트 이름을, `api/index.ts`에 `getHookInstallation`, `onHookEvent`, `onHookInstallation`을 추가합니다. 확인: `pnpm typecheck` 통과 (FR-HOOK-009, FR-HOOK-010)

### 8. 화면

- [x] 8.1 `hookEventsView.ts`의 시그니처를 만들고 `test-writer`가 `appendEvent`(200개, 201번째에서 첫 이벤트 빠짐, 입력 불변), `toEventRow`, `toInstallRows` 테스트를 씁니다. 확인: 새 테스트가 실패함 (FR-HOOK-010)
- [x] 8.2 `hookEventsView.ts`를 구현합니다. 확인: `pnpm test` 통과 (FR-HOOK-010)
- [x] 8.3 `HookEventsPanel.tsx`를 만들고 `App.tsx`의 `main` 창에 붙입니다. 확인: `pnpm dev`에서 설치 결과가 보이고, 터미널의 Claude Code에 질문하면 `UserPromptSubmit`부터 `Stop`까지 1초 안에 목록에 추가됨 (FR-HOOK-010, SC-HOOK-002)

### 9. 실제 환경 확인

- [x] 9.1 macOS: Claude Code와 Codex에 질문해 이벤트가 화면에 나오는지, 기존 Orca와 iTerm2 훅이 그대로 동작하는지, 앱을 끈 상태에서 훅이 2초 안에 끝나는지, 앱을 다시 켜면 설정 파일이 바뀌지 않는지 확인합니다 (FR-HOOK-002, FR-HOOK-004, FR-HOOK-005, FR-HOOK-008, SC-HOOK-001)
- [x] 9.2 Windows: Windows에 바로 설치한 Claude Code(Git Bash 있음·없음)와 Codex, WSL 배포판의 Codex에 질문해 이벤트가 나오는지, 공백·한글 사용자 폴더에서도 되는지, 콘솔 창이 뜨지 않는지 확인합니다 (FR-HOOK-008, FR-HOOK-011)
