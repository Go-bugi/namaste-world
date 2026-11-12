# Plan

## 설계

### 층 배치

| 층 | 파일 | 하는 일 |
| --- | --- | --- |
| core(Rust) | `src-tauri/src/agent/domain/model.rs` | 감지 결과 타입(에이전트, 실행 환경, 상태, 실패 이유) |
| core(Rust) | `src-tauri/src/agent/domain/parse.rs` | WSL 배포판 목록, 버전 출력, 프로브 출력, 로그인 셸 `PATH` 출력 해석 |
| core(Rust) | `src-tauri/src/agent/domain/assemble.rs` | 한 환경의 프로브 결과를 항목 목록으로 바꾸는 규칙, 실행 파일을 찾을 디렉터리 목록 |
| IO(Rust) | `src-tauri/src/agent/host.rs` | 환경 목록과 환경별 프로브를 돌려주는 `Host` trait |
| IO(Rust) | `src-tauri/src/agent/detect.rs` | `Host`로 환경을 모두 확인하고 10초 상한 안에 결과를 모음 |
| IO(Rust) | `src-tauri/src/platform/macos.rs`, `windows.rs`, `mod.rs` | `Host` 구현. macOS 로그인 셸 `PATH`, Windows `PATH`와 설치 폴더, `wsl.exe` 실행. 그 밖의 OS는 환경 없음 |
| 공유 상태(Rust) | `src-tauri/src/state.rs` | 보관한 감지 결과(`tokio::sync::watch`) |
| 백그라운드 작업 | `src-tauri/src/tasks/agent_detection.rs` | 앱 시작 때 감지(`Trigger::Startup`)를 한 번 돌려 상태에 넣음 |
| 명령 | `src-tauri/src/commands/agent.rs`, `lib.rs` | `agent_detection`, `redetect_agents` 명령 등록, `setup`에서 상태와 작업 등록 |
| IO(TS) | `src/renderer/src/api/index.ts` | 두 명령을 `call`로 감싼 함수 |
| core(TS) | `src/renderer/src/features/agent-detection/detectionView.ts` | 감지 결과를 화면 줄로 바꾸는 함수, 화면 상태 전이 |
| React | `src/renderer/src/features/agent-detection/AgentDetectionPanel.tsx` | 목록과 다시 감지 버튼 |
| 화면 조립 | `src/renderer/src/windows/main/App.tsx` | `main` 창에 패널 추가 |

- `agent/`는 IO가 섞인 로직 모듈이라 순수한 부분을 `agent/domain/`에 둡니다. `agent/`는 `tauri`를 import하지 않습니다.
- `#[cfg(target_os)]` 코드는 `platform/`에만 둡니다. `agent/detect.rs`는 `Host` trait만 압니다.
- TS core 파일은 하나라서 `domain/` 폴더를 만들지 않습니다. 이 화면에서만 쓰는 상태라 store를 만들지 않고 컴포넌트 상태와 `detectionView.ts`의 전이 함수로 다룹니다.

### 타입과 경계

ts-rs로 내보내는 타입(`src/renderer/src/types/bindings/`에 생성):

```rust
#[serde(rename_all = "camelCase")]
pub enum AgentKind { Claude, Codex }

#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Environment { Macos, Windows, Wsl { distro: String } }

#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FailureReason {
    Timeout,                              // 시간 안에 확인하지 못함
    VersionUnreadable,                    // 실행 파일은 있으나 --version 실패
    EnvironmentError { message: String }, // 환경 자체를 확인하지 못함(wsl.exe 실패 등)
}

#[serde(tag = "status", rename_all = "camelCase")]
pub enum AgentStatus {
    Installed { path: String, version: String },
    NotInstalled,
    Stopped,                              // 멈춘 WSL 배포판이라 앱 시작 감지에서 확인하지 않음
    Failed { reason: FailureReason },
}

pub struct AgentEntry { pub agent: AgentKind, pub environment: Environment, pub status: AgentStatus }
pub struct Detection { pub entries: Vec<AgentEntry> }
```

Rust 명령(둘 다 실패하지 않으므로 `Result`가 아닌 `Detection`을 돌려줍니다):

| 명령 | 동작 |
| --- | --- |
| `agent_detection` | 보관한 결과가 있으면 바로 주고, 앱 시작 감지가 아직이면 끝날 때까지 기다렸다가 줍니다(FR-AGENT-007) |
| `redetect_agents` | `Trigger::UserRequested`로 새로 감지해(멈춘 배포판도 켬) 보관한 결과를 바꾸고 그 결과를 줍니다. 진행 중에도 `agent_detection`은 이전 결과를 바로 줍니다 |

- 이벤트, 플러그인, capability 권한은 추가하지 않습니다. 앱이 만든 명령은 기본으로 허용됩니다.
- 의존성: `src-tauri/Cargo.toml`에 `tokio = { version = "1", features = ["process", "time", "sync", "macros", "rt"] }`를 추가합니다. Tauri가 이미 tokio 1.x를 쓰므로 새 크레이트가 늘지는 않습니다. 테스트용으로 `[dev-dependencies]`에 `tokio`의 `test-util`을 켭니다(시간 멈추기).

### 시그니처

core(Rust, `agent/domain/`):

```rust
/// `wsl.exe -l -q` 출력에서 배포판 이름을 뽑는다.
/// 출력은 UTF-16LE(BOM 있을 수 있음)이거나 UTF-8이다. 빈 줄과 NUL은 버리고,
/// 도구용 배포판(`docker-desktop`, `docker-desktop-data`)은 뺀다. 순서는 출력 순서 그대로.
pub fn parse_distro_list(raw: &[u8]) -> Vec<String>;

/// `--version` 출력에서 버전 문자열을 뽑는다. 첫 줄을 공백으로 나눈 토큰 가운데, 앞의 `v` 하나를 뗀 뒤
/// `숫자.숫자.숫자`로 시작하는 첫 토큰 전체. "v2.1.322" → "2.1.322", "2.1.322-beta.1" → 그대로, "2.1" → None
/// 예: "2.1.322 (Claude Code)" → Some("2.1.322"), "codex-cli 0.146.0" → Some("0.146.0"), "" → None
pub fn parse_version(output: &str) -> Option<String>;

/// 로그인 셸에서 `printf '<marker>%s<marker>' "$PATH"`로 찍은 출력에서 PATH를 뽑는다.
/// 셸 설정 파일이 찍은 다른 문구는 무시한다. 처음 두 마커 사이를 쓰고, 마커가 두 번 나오지 않으면 None.
pub fn parse_marked_path(stdout: &str, marker: &str) -> Option<String>;

/// WSL 프로브 스크립트 출력을 에이전트별 결과로 바꾼다.
/// 줄 형식: `<marker>\t<agent>\t<path>\t<version 출력 첫 줄>`. 마커 없는 줄과 POSIX 절대 경로가 아닌 줄은 버린다.
/// 없는 에이전트는 맵에 넣지 않는다. 같은 에이전트가 여러 줄이면 첫 줄(PATH에서 먼저 찾은 것)을 쓴다.
pub fn parse_probe_output(stdout: &str, marker: &str) -> BTreeMap<AgentKind, Found>;

/// 실행 파일 하나를 찾은 결과. version_output이 None이면 --version이 실패했거나 시간 안에 끝나지 않았다.
pub struct Found { pub path: String, pub version_output: Option<String> }

/// 한 환경의 프로브 결과. Err면 환경 자체를 확인하지 못했다.
pub type EnvProbe = Result<BTreeMap<AgentKind, Found>, FailureReason>;

/// 감지를 누가 시작했는가. 사용자가 요청한 감지만 멈춘 WSL 배포판을 켤 수 있다.
pub enum Trigger { Startup, UserRequested }

/// 확인할 실행 환경과, 그 환경이 지금 실행 중인지. macOS와 Windows는 항상 running: true.
pub struct Target { pub environment: Environment, pub running: bool }

/// 이 환경을 프로브할지. running이면 true, 멈췄으면 UserRequested일 때만 true.
pub fn should_probe(target: &Target, trigger: Trigger) -> bool;

/// 프로브하지 않은 환경의 항목. 항상 Claude, Codex 순서로 두 항목 모두 Stopped.
pub fn stopped_entries(environment: &Environment) -> Vec<AgentEntry>;

/// 한 환경의 프로브 결과를 항목으로 바꾼다. 항상 Claude, Codex 순서로 두 항목을 낸다.
/// Err(reason) → 두 항목 모두 Failed(reason) / 맵에 없음 → NotInstalled /
/// 버전을 뽑지 못함 → Failed(VersionUnreadable) / 그 밖 → Installed
pub fn entries_for(environment: &Environment, probe: &EnvProbe) -> Vec<AgentEntry>;

/// 실행 파일을 찾을 디렉터리. PATH 항목을 먼저, 설치 방법별 기본 폴더를 뒤에 두고 중복은 뺀다.
/// macOS 기본 폴더: ~/.local/bin, ~/.claude/local, /opt/homebrew/bin, /usr/local/bin, ~/.npm-global/bin, ~/.bun/bin, ~/.volta/bin
/// Windows 기본 폴더: %USERPROFILE%\.local\bin, %APPDATA%\npm, %LOCALAPPDATA%\Microsoft\WinGet\Links
/// 기본 폴더는 OS와 무관하게 `Path::join`으로 만든다.
pub fn search_dirs(os: HostOs, path_env: &str, home: &Path, app_data: Option<&Path>, local_app_data: Option<&Path>) -> Vec<PathBuf>;

/// 디렉터리에서 찾아볼 파일 이름. macOS는 [name], Windows는 [name.exe, name.cmd].
pub fn executable_names(os: HostOs, agent: AgentKind) -> Vec<String>;

pub enum HostOs { Macos, Windows }
```

IO(Rust):

```rust
/// 환경 목록과 환경별 프로브. platform/이 OS마다 구현한다. 테스트는 가짜 구현을 쓴다.
pub trait Host: Sync {
    /// 확인할 실행 환경. macOS는 [Macos], Windows는 [Windows, Wsl{..}...].
    /// WSL 배포판의 running은 `wsl.exe -l --running -q`에 있는지로 정한다. 목록을 읽지 못하면 WSL 없이 [Windows].
    fn targets(&self) -> impl Future<Output = Vec<Target>> + Send;
    fn probe(&self, environment: &Environment) -> impl Future<Output = EnvProbe> + Send;
}

/// 모든 환경을 동시에 확인하고 limit 안에 결과를 낸다.
/// should_probe가 false인 환경은 probe를 부르지 않고 stopped_entries로 채운다(배포판을 켜지 않음).
/// limit 안에 끝나지 않은 환경의 항목은 Failed(Timeout). 한 환경의 실패는 다른 환경에 영향이 없다.
/// 항목 순서는 targets() 순서, 그 안에서 Claude, Codex 순서.
pub async fn detect<H: Host>(host: &H, trigger: Trigger, limit: Duration) -> Detection;

pub const DETECTION_LIMIT: Duration = Duration::from_secs(10);

/// state.rs. 마지막으로 끝난 감지 결과를 보관한다. Tauri 없이 쓸 수 있다.
pub struct AgentDetection { /* watch::Sender<Option<Detection>> */ }
impl AgentDetection {
    /// 아직 결과가 없는 상태.
    pub fn new() -> Self;
    /// 보관한 결과. 없으면 첫 publish까지 기다린다. 있으면(다시 감지 중이어도) 바로 준다.
    pub async fn latest(&self) -> Option<Detection>;
    /// 보관한 결과를 바꾼다. 기다리던 latest()도 이 값을 받는다.
    pub fn publish(&self, detection: Detection);
}

#[tauri::command] pub async fn agent_detection(state: State<'_, AgentDetection>) -> Result<Detection, ()>;
#[tauri::command] pub async fn redetect_agents(state: State<'_, AgentDetection>) -> Result<Detection, ()>;
```

`async` 명령이 `State`를 받으려면 Tauri가 `Result` 반환을 요구해서 에러 타입을 `()`로 둡니다. TS에서는 `ResultAsync<Detection, IpcError>`가 됩니다.

core(TS, `features/agent-detection/detectionView.ts`):

```ts
/** 화면 한 줄. label은 사용자에게 보이는 문구다. */
export type DetectionRow = {
  readonly key: string; // `${agent}:${환경 kind}:${distro ?? ''}`
  readonly agentLabel: 'Claude Code' | 'Codex';
  readonly environmentLabel: string; // 'macOS' | 'Windows' | `WSL · ${distro}`
  readonly statusLabel: '설치됨' | '설치되지 않음' | '확인 실패' | '멈춤';
  readonly detail?: string; // 설치됨: `${version} · ${path}`, 확인 실패: 이유 문구, 멈춤: '다시 감지하면 배포판을 켜서 확인합니다', 설치되지 않음: 없음
};

/** 감지 결과를 entries 순서 그대로 화면 줄로 바꾼다. */
export function toRows(detection: Detection): readonly DetectionRow[];

/** 실패 이유 문구. timeout → '시간 초과', versionUnreadable → '버전 확인 실패', environmentError → message */
export function failureLabel(reason: FailureReason): string;

/** 화면 상태. result가 없으면 처음 감지 중이다. */
export type PanelState = { readonly result?: Detection; readonly detecting: boolean };
export type PanelEvent =
  | { readonly type: 'redetectStarted' }
  | { readonly type: 'detected'; readonly result: Detection }
  | { readonly type: 'failed' };

export const initialPanelState: PanelState; // { detecting: true }
/** redetectStarted → 이전 result를 두고 detecting: true / detected → result 교체, detecting: false / failed → result 유지, detecting: false */
export function panelReducer(state: PanelState, event: PanelEvent): PanelState;
/** 다시 감지 버튼을 누를 수 있는가. detecting이면 false */
export function canRedetect(state: PanelState): boolean;
```

### 결정

- **Orca 방식을 기본으로 따릅니다.** Orca(`stablyai/orca`의 `src/main/preflight/`)와 Paseo(`getpaseo/paseo`의 `packages/server/src/executable-resolution/`)를 확인했습니다.
  - 로그인 셸 `PATH`: 둘 다 `$SHELL -ilc`로 마커를 찍어 셸 설정 파일이 찍는 문구와 섞이지 않게 읽습니다(Paseo는 VS Code의 `shellEnv.ts`를 가져왔습니다). macOS에서 같은 방식을 씁니다(FR-AGENT-004).
  - 설치 폴더 보충: Orca는 `PATH`에 없을 때 `~/.local/bin`, npm, pnpm, bun, volta 같은 설치 폴더를 직접 봅니다. Paseo는 Windows에서 WinGet 패키지 폴더를 더 봅니다. 두 목록을 합쳐 `search_dirs`에 둡니다(FR-AGENT-001 "설치 방법과 무관").
  - WSL: Orca는 배포판마다 `wsl.exe`로 `sh` 스크립트 하나를 실행해 모든 명령을 한 번에 찾고, 결과 줄에 마커를 붙이며, `/mnt/<드라이브>` 아래(Windows 쪽 실행 파일)는 건너뜁니다. 같은 방식을 쓰고 버전 확인도 같은 스크립트에서 합니다. 배포판 하나에 `wsl.exe`를 한 번만 띄워 꺼진 배포판을 켜는 비용을 한 번으로 줄입니다.
  - 버전 확인: Paseo는 후보마다 `--version`을 3초 제한으로 실행해 실제로 뜨는 실행 파일만 고릅니다. 같은 기준으로 버전을 읽지 못하면 확인 실패로 냅니다(clarify Q3).
  - Paseo는 WSL 안의 에이전트를 감지하지 않아서 WSL은 Orca만 참고했습니다.
- **멈춘 배포판은 사용자가 요청할 때만 켭니다(clarify Q10).** Orca는 `wsl.exe -d`가 멈춘 배포판을 켠다는 이유로 백그라운드 작업 전에 실행 중인지 확인하고, 멈췄으면 아무것도 실행하지 않습니다(`wsl/discovery/wsl-background-exec-gate.ts`). 같은 규칙을 `should_probe`에 둡니다.
- **WSL 명령**: `wsl.exe -l -q`로 배포판 목록을, `wsl.exe -l --running -q`로 실행 중인 배포판을 읽고, `wsl.exe -d <배포판> -e sh -lc '<스크립트>'`로 확인합니다. `WSL_UTF8=1`을 주고, 그래도 UTF-16LE로 오는 경우를 `parse_distro_list`가 처리합니다. 스크립트는 `PATH` 앞에 `~/.local/bin`, `~/.npm-global/bin`, `~/.bun/bin`, `~/.volta/bin`, nvm의 node bin을 덧붙입니다. `sh -lc`는 `.bashrc`를 읽지 않아 그 파일에만 `PATH`를 넣은 사용자를 놓치기 때문입니다.
- **Windows 콘솔 창**: `wsl.exe`와 `--version`을 실행할 때 `CREATE_NO_WINDOW`를 줍니다. 주지 않으면 실행할 때마다 콘솔 창이 깜박입니다.
- **시간 상한**: `detect`가 시작 시각 + 10초를 마감으로 잡고 환경별 프로브를 `tokio::time::timeout_at`으로 감쌉니다. 마감을 넘긴 환경은 `Timeout`입니다. 프로세스는 `kill_on_drop(true)`로 띄워 시간 초과 때 정리합니다. 개별 `--version`은 3초 제한입니다.
- **결과 보관**: `tokio::sync::watch::Sender<Option<Detection>>` 하나를 상태로 둡니다. `agent_detection`은 값이 `Some`이면 바로 주고 `None`이면 바뀔 때까지 기다립니다. `redetect_agents`는 감지가 끝날 때만 값을 바꾸므로 그동안 다른 요청은 이전 결과를 받습니다(clarify Q8). 메모리에만 두고 디스크에 쓰지 않습니다.
- **화면 갱신**: 이벤트 대신 명령의 반환값으로 화면을 바꿉니다. 화면이 감지를 시작하는 유일한 곳이라 이벤트가 필요 없습니다. 감지가 겹쳐 요청되는 경우는 clarify Q9에 따라 이번에 다루지 않고, 지금은 요청마다 각자 감지하고 늦게 끝난 결과가 남습니다.
- 버린 방안: Rust 크레이트 `which`는 `PATHEXT`와 `PATH` 검색만 해 주고 설치 폴더 보충과 버전 확인은 결국 직접 해야 해서 쓰지 않습니다. 감지를 renderer(TS)에서 하는 방안은 renderer가 OS에 직접 접근할 수 없어서 버립니다.

### 위험

- [위험] 사용자 셸 설정 파일이 느리거나 입력을 기다려 `$SHELL -ilc`가 끝나지 않음 → Orca(`startup/hydrate-shell-path.ts`)처럼 다룹니다. stdin을 닫고 stderr를 버리며, 새 프로세스 그룹으로 띄워 앱을 띄운 터미널을 차지하지 못하게 합니다. 3초 제한을 넘기면 죽이고 앱 `PATH`와 기본 폴더만으로 찾습니다. 읽은 `PATH`는 앱이 켜져 있는 동안 재사용하고, 다시 감지할 때만 새로 읽습니다.
- [위험] 다시 감지에서 멈춘 배포판을 켜는 데 10초 가까이 걸림 → 그 배포판만 `Timeout`으로 나오고 Windows 결과는 정상으로 나옵니다(FR-AGENT-005). 한 번 더 누르면 배포판이 이미 켜져 있어 빨리 끝납니다. 앱 시작 감지는 배포판을 켜지 않으므로 이 위험이 없습니다.
- [위험] 시간 초과로 `wsl.exe`를 죽여도 배포판 안의 프로세스가 남을 수 있음 → 스크립트가 `command -v`와 `--version`만 실행해 곧 끝나므로 받아들입니다.
- [위험] CI는 Linux에서 돌아 `platform/macos.rs`, `windows.rs`가 CI에서 컴파일되지 않음 → 판정과 해석은 모두 OS와 무관한 `agent/domain/`에 두어 CI 테스트로 확인하고, OS별 코드는 얇게 둡니다. macOS와 Windows 동작은 사용자가 직접 확인합니다.
- [위험] npm이 만든 `claude.cmd`를 Rust `Command`로 실행할 때 인자 처리 → 인자가 `--version` 하나뿐이라 영향이 없습니다.

## 작업

### 1. 감지 결과 타입과 해석 (core)

- [x] 1.1 `tokio` 의존성을 추가하고 `agent/domain/model.rs`에 결과 타입을 ts-rs derive와 함께 정의합니다. 확인: `pnpm test:rust`가 `types/bindings/`에 TS 타입을 만듦 (FR-AGENT-003)
- [x] 1.2 `parse_distro_list`, `parse_version`, `parse_marked_path`, `parse_probe_output`의 시그니처를 본문 없이(`todo!()`) 만들고 `test-writer`가 테스트를 씁니다. 확인: 새 테스트가 실패함 (FR-AGENT-002, FR-AGENT-003, FR-AGENT-004)
- [x] 1.3 1.2의 네 함수를 구현합니다. 확인: 1.2 테스트 통과 (FR-AGENT-002, FR-AGENT-003, FR-AGENT-004)
- [x] 1.4 `entries_for`, `should_probe`, `stopped_entries`, `search_dirs`, `executable_names`를 시그니처만 만들고 `test-writer`가 테스트를 쓴 뒤 구현합니다. 확인: 테스트 통과 (FR-AGENT-001, FR-AGENT-002, FR-AGENT-003, FR-AGENT-005)

### 2. 감지 흐름과 시간 상한

- [x] 2.1 `agent/host.rs`의 `Host` trait과 `agent/detect.rs`의 `detect` 시그니처를 만들고, `test-writer`가 가짜 `Host`로 테스트를 씁니다(환경별 실패 격리, 멈춘 프로브의 10초 시간 초과, 순서, 앱 시작 감지에서 멈춘 배포판의 probe를 부르지 않음). 확인: 새 테스트가 실패함 (FR-AGENT-002, FR-AGENT-005, SC-AGENT-001)
- [x] 2.2 `detect`를 구현합니다. 확인: 2.1 테스트 통과 (FR-AGENT-002, FR-AGENT-005, SC-AGENT-001)

### 3. OS별 구현

- [x] 3.1 `platform/macos.rs`에 `Host`를 구현합니다(로그인 셸 `PATH` 읽기와 재사용, `search_dirs`, `--version` 실행). 그 밖의 OS(Linux)는 `platform/mod.rs`에서 환경 없음으로 둡니다. 확인: macOS에서 `pnpm lint:rust` 통과, Finder로 띄운 빌드에서 `claude` 감지 (FR-AGENT-001, FR-AGENT-004)
- [x] 3.2 `platform/windows.rs`에 `Host`를 구현합니다(`PATH`/설치 폴더, `wsl.exe -l -q`와 `-l --running -q`, 배포판별 프로브 스크립트, `CREATE_NO_WINDOW`). 확인: `cargo check --target x86_64-pc-windows-msvc`가 되면 통과, 아니면 사용자가 Windows에서 확인 (FR-AGENT-001, FR-AGENT-002, FR-AGENT-005)

### 4. 보관과 명령

- [x] 4.1 `test-writer`가 `AgentDetection` 테스트를 쓴 뒤 `state.rs`에 보관 상태를, `tasks/agent_detection.rs`에 앱 시작 감지를, `commands/agent.rs`에 두 명령을 두고 `lib.rs`의 `setup`과 `generate_handler!`에 등록합니다. 확인: `pnpm test:rust`, `pnpm lint:rust` 통과 (FR-AGENT-006, FR-AGENT-007)
- [x] 4.2 `api/index.ts`에 `getAgentDetection`, `redetectAgents`를 추가합니다. 확인: `pnpm typecheck` 통과 (FR-AGENT-007)

### 5. 목록 화면

- [x] 5.1 `detectionView.ts`의 시그니처를 만들고 `test-writer`가 `toRows`, `failureLabel`, `panelReducer`, `canRedetect` 테스트를 씁니다. 확인: 새 테스트가 실패함 (FR-AGENT-008)
- [x] 5.2 `detectionView.ts`를 구현합니다. 확인: `pnpm test` 통과 (FR-AGENT-008)
- [x] 5.3 `AgentDetectionPanel.tsx`를 만들고 `App.tsx`의 `main` 창에 붙입니다. 확인: `pnpm dev`에서 목록, 확인 중 표시, 감지 중 버튼 비활성화, 다시 감지 후 갱신이 보임 (FR-AGENT-006, FR-AGENT-008)
