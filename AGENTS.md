# AGENTS.md

이 저장소에서 작업하는 코딩 에이전트(Claude Code, Codex 등)가 공통으로 읽는 안내입니다. 에이전트별 설정 파일(`CLAUDE.md` 등)은 이 파일을 불러오고, 그 에이전트에만 해당하는 내용만 덧붙입니다.

## 명령어

```bash
pnpm dev              # Vite dev 서버(HMR) + Rust 빌드 + 앱 실행. Rust 코드가 바뀌면 앱을 다시 빌드해 재시작
pnpm typecheck        # typecheck:node(vite.config.ts) + typecheck:web(renderer)
pnpm lint             # ESLint (lint:fix로 자동 수정)
pnpm lint:rust        # cargo clippy (경고도 실패로 처리)
pnpm format:check     # Prettier 검사 (format으로 자동 수정)
pnpm format:check:rust # rustfmt 검사 (format:rust로 자동 수정)
pnpm test             # Vitest 한 번 실행 (test:watch는 watch 모드)
pnpm test:rust        # cargo test
pnpm sdd:check        # SDD 상태 판정 스크립트 테스트 (node:test)
pnpm build            # typecheck 후 renderer만 out/renderer에 빌드
pnpm build:unpack     # 설치 파일 없이 앱 실행 파일만 빌드 (src-tauri/target/release/)
pnpm build:mac        # dmg 패키징. build:win은 nsis 설치 파일, build:win:arm64는 Windows ARM용 nsis 설치 파일
```

- 테스트는 TS가 Vitest(+ fast-check), Rust가 `cargo test`입니다. TS 테스트는 소스 옆에 `*.test.ts(x)`로, Rust 테스트는 같은 파일의 `#[cfg(test)] mod tests`에 둡니다. Vitest 설정은 `vite.config.ts`의 `test`에 있습니다.
- core(순수 함수)를 새로 만들거나 바꿀 때는 테스트를 먼저 씁니다. 버그를 고칠 때는 그 버그를 재현하는 테스트부터 씁니다. 원인이 shell(컴포넌트, store, 명령)에 있으면 원인이 되는 계산을 core로 옮긴 뒤 그 테스트를 씁니다.
- PR CI(`pr-check.yml`)는 `format:check`, `lint`, `test`, `build`, `format:check:rust`, `lint:rust`, `test:rust`를 돌립니다. SDD 검사(`sdd:check`, OpenSpec schema와 스펙 형식 검사)는 별도 job으로 돕니다.
- Node 24.21.0 이상이 필요합니다(`.nvmrc`). `pnpm-workspace.yaml`의 `engineStrict` 때문에 Node 버전이 낮으면 `pnpm run`과 `pnpm install`이 실패하니, 먼저 `nvm use`를 실행하세요.
- Rust stable 툴체인이 필요합니다(`Cargo.toml`의 `rust-version` 이상). `rustup update stable`로 맞춥니다.
- 커밋할 때 husky가 lint-staged로 ESLint, Prettier, rustfmt를 적용합니다. `commit-msg` hook은 없어서 커밋 메시지는 검사하지 않습니다.

## 구조

[Tauri 2](https://v2.tauri.app/)를 씁니다. 화면은 React 앱(`src/renderer`)이고, OS 기능과 창 관리는 Rust(`src-tauri`)가 맡습니다.

| 영역     | 위치           | 빌드                                       | 출력                                                                                |
| -------- | -------------- | ------------------------------------------ | ----------------------------------------------------------------------------------- |
| renderer | `src/renderer` | `vite.config.ts`                           | `out/renderer`                                                                      |
| 앱(Rust) | `src-tauri`    | `cargo`. Tauri CLI가 `tauri.conf.json`대로 | `src-tauri/target/release/`(`--target`을 주면 `src-tauri/target/<target>/release/`) |

- **Tauri CLI가 Vite를 부릅니다.** `pnpm dev`(`tauri dev`)는 `beforeDevCommand`로 `pnpm dev:web`(Vite, 5173 `strictPort`)를 띄우고 그 주소를 웹뷰에 엽니다. `tauri build`는 `beforeBuildCommand`로 `pnpm build`를 돌린 뒤 `out/renderer`를 실행 파일에 넣습니다.
- **웹뷰는 OS마다 다릅니다.** macOS는 WKWebView(Safari 계열), Windows는 WebView2(Chromium)입니다. CSS와 Web API는 Safari에서도 동작하는지 확인하고, `vite.config.ts`의 `build.target`도 플랫폼에 따라 다르게 잡습니다.
  - macOS 빌드 타깃은 `safari13`입니다. esbuild는 문법만 낮추고 메서드는 polyfill하지 않으므로 Safari 13에 없는 API는 타입 검사를 통과해도 런타임 오류가 납니다. `toSorted`, `toReversed`, `toSpliced`, `with`, `findLast`, `findLastIndex`, `at`, `Object.hasOwn`, `structuredClone`, `replaceAll`, `Promise.any`, `crypto.randomUUID`, `requestIdleCallback`, CSS `content-visibility`는 쓰지 않습니다. CSS를 뺀 나머지는 ESLint(`no-restricted-properties`, `no-restricted-globals`)가 `src/renderer`에서 검사합니다. 함수형 코드에서 쓸 대안은 `fp-apply` 스킬의 `references/typescript.md`에 있습니다.
- **renderer는 OS에 직접 접근할 수 없습니다.** Rust 명령이나 플러그인을 거쳐야 하고, 쓸 수 있는 기능은 `src-tauri/capabilities/*.json`에 선언한 권한으로 제한됩니다. 기능을 추가할 때는 아래를 함께 고칩니다.
  1. `src-tauri/src/commands/`에 `#[tauri::command]` 함수를 만들고 `lib.rs`의 `generate_handler!`에 등록합니다. 인자와 반환값, 에러 타입에는 ts-rs derive를 붙이고 `pnpm test:rust`로 TS 타입을 만듭니다(아래 "Rust와 TS 타입 맞추기").
  2. `src/renderer/src/api/`의 `api`에 그 명령을 `invoke`로 감싸는 함수를 추가합니다. 컴포넌트에서 `invoke`를 직접 부르지 않습니다. 함수는 같은 파일의 `call`로 감싸 `ResultAsync<반환 타입, 명령 에러 | IpcError>`를 돌려줍니다. reject 값은 Rust 명령 에러(`kind`가 있는 객체)일 수도 있고, 없는 명령이나 인자 불일치처럼 Tauri가 보낸 문자열일 수도 있어서 `call`이 둘을 가릅니다.
  3. 플러그인이나 코어 API를 쓰면 그 기능을 쓰는 창의 capability 파일(`src-tauri/capabilities/<창 이름>.json`)에서 `permissions`에 필요한 권한만 추가합니다.
- **CSP는 `tauri.conf.json`의 `app.security.csp`에서 설정합니다.** `index.html`에 `<meta>`로 넣지 않습니다. IPC가 `ipc:`/`http://ipc.localhost`를 쓰므로 `connect-src`에서 이 둘을 빼면 안 됩니다.
- **tsconfig는 둘로 나뉩니다.** `tsconfig.node.json`은 `vite.config.ts`를, `tsconfig.web.json`은 renderer를 다룹니다. renderer에서는 `@renderer/*` 별칭으로 `src/renderer/src`를 가리킵니다.
- 앱 ID는 `tauri.conf.json`의 `identifier`(`com.namasteworld.app`)입니다. Windows 알림과 작업 표시줄 그룹에도 이 값이 쓰입니다.
- **앱 버전은 `package.json` 하나에서 옵니다.** `tauri.conf.json`의 `version`이 `"../package.json"`을 가리킵니다. `Cargo.toml`의 `version`은 `0.0.0`으로 고정하고 올리지 않습니다. 올리면 브랜치마다 값이 달라져 back-merge 때 `Cargo.toml`, `Cargo.lock`이 충돌합니다.
- 렌더러 의존성은 Vite가 번들하므로 지금처럼 `devDependencies`에 추가합니다. Rust 의존성은 `src-tauri/Cargo.toml`에 추가하고 `Cargo.lock`도 커밋합니다(CI가 `--locked`로 검사).
- 기획 문서는 `design/`에 둡니다.

## 폴더 구조

코드는 아래 구조에 맞춰 둡니다.

```
src/renderer/
  index.html            # main 창 진입 HTML. 창이 늘면 <창 이름>.html을 추가
  public/               # 웹뷰가 URL로 읽는 정적 파일 (models/, draco/, hdr/, sounds/)
  src/
    windows/<창 이름>/   # 창별 진입점(main.tsx, App.tsx). 화면 조립만 함
    scene/              # R3F(Canvas 안) 코드
    features/<기능>/     # 기능 단위 UI, Hook, 그 기능에서만 쓰는 store
      domain/           # 그 기능의 순수 로직(core)과 테스트. core 파일이 여럿일 때만 만듦
    stores/             # 여러 곳이 같이 쓰는 zustand store
    api/                # Rust 명령과 이벤트를 감싼 함수. invoke, listen은 여기서만 부름
    types/
      ipc.ts            # 이벤트 이름 상수
      bindings/         # ts-rs가 Rust 타입에서 만든 TS 타입. 직접 고치지 않음
    shared/             # 두 기능 이상이 쓰는 코드 (ui/, lib/)
    dev/                # 개발 전용 코드 (leva-stub 등)
    assets/             # import해서 번들에 넣는 CSS, 작은 이미지

src-tauri/
  src/
    main.rs
    lib.rs              # Builder 조립만 함
    error.rs            # 명령이 돌려주는 공통 에러 타입
    state.rs            # app.manage()로 등록하는 상태
    events.rs           # 이벤트 이름 상수와 payload 타입
    commands/           # #[tauri::command] 함수
    tasks/              # 계속 도는 백그라운드 작업. setup에서 띄움
    window/<창 이름>.rs  # 그 창의 이벤트 처리와 설정 코드. 할 일이 없으면 만들지 않음
    platform/           # OS별 코드 (macos.rs, windows.rs)
    tray.rs
    ai/, db/ …          # 앱 로직 모듈
      domain/           # 그 모듈의 순수 로직(core). IO가 섞인 모듈에만 만듦
  capabilities/<창 이름>.json
  resources/            # Rust가 읽는 파일. tauri.conf.json의 bundle.resources에 등록
```

### 원칙

- 폴더는 넣을 파일이 생길 때 만듭니다. 빈 폴더나 `.gitkeep`만 있는 폴더는 만들지 않습니다.
- `windows/<창 이름>/`에서는 화면을 조립만 합니다. 로직은 `scene/`, `features/`, `shared/lib/`에 두고, 여러 곳이 같이 쓰는 상태는 `stores/`에 둡니다.
- 함수형으로 나눈 층은 아래 위치에 둡니다. 층을 나누는 원칙은 `fp-apply` 스킬에 있습니다.
  - **core**(순수 함수, 도메인 타입): 그 코드를 쓰는 컴포넌트, Hook, store 가까이 둡니다. 기능 하나면 `features/<기능>/`, 3D면 `scene/`, 여러 곳이 쓰면 `shared/lib/`입니다. Rust는 로직 모듈 안의 IO 없는 코드입니다.
  - **IO 경계**: TS는 `api/`이고 주고받는 타입은 `types/bindings/`(Rust에서 생성)입니다. Rust는 `commands/`와, 로직 모듈 중 네트워크, 파일, DB에 접근하는 부분(`ai/client.rs`, `db/` 등)입니다.
  - **공유 상태**: TS는 `stores/`, Rust는 `state.rs`입니다. Rust 명령의 공통 에러는 `error.rs`에 둡니다.
- **core는 `domain/` 하위 폴더에 모읍니다.** TS와 Rust 모두 이름은 `domain/`입니다. Rust에서 `core`는 표준 크레이트 이름과 겹치기 때문입니다.
  - TS는 `features/<기능>/`이나 `scene/`에 core 파일이 둘 이상이면 `domain/`을 만들고, 하나면 그 폴더에 바로 둡니다. `shared/lib/`는 전체가 core라서 `domain/`을 두지 않습니다.
  - Rust는 로직 모듈에 IO가 섞여 있으면 `<모듈>/domain/`을 만듭니다(`ai/domain/prompt.rs`와 `ai/client.rs`). 모듈 전체가 순수하면 그 모듈 자체가 core라서 `domain/`을 두지 않습니다.
  - Rust 파일 이름은 snake_case 모듈 이름을 그대로 씁니다. `_core` 같은 접미사는 붙이지 않습니다.
  - `domain/` 안의 파일은 React, zustand, three(`@react-three/*` 포함), Tauri(`@tauri-apps/*`, `tauri`), `api/`, IO 모듈을 import하지 않습니다. ESLint가 TS 쪽을 검사합니다. `types/bindings/`의 타입은 `import type`으로 써도 됩니다. 테스트는 `domain/` 안에 대상 파일 옆으로 둡니다.
- `scene/`과 `features/`는 서로 import하지 않습니다. 둘이 같이 쓰는 상태는 `stores/`에 둡니다.
- 기능 하나에서만 쓰는 코드와 store는 `features/<기능>/` 안에 둡니다. 두 기능 이상이 쓰게 되면 코드는 `shared/`로, store는 `stores/`로 옮깁니다.
- `hooks/`, `utils/`, `constants/` 같은 최상위 폴더는 만들지 않습니다. 기능 폴더나 `shared/` 안에 둡니다.
- Rust는 shell(`commands/`, `tasks/`, `lib.rs`의 `setup`)이 로직 모듈(`ai/`, `db/` 등)을 부르는 방향으로만 의존합니다. 로직 모듈은 `tauri`를 import하지 않습니다. 계산은 IO 없는 함수로 두고, 파일이나 DB에 접근하는 코드는 계산 쪽이 정의한 trait을 구현하는 모듈(`db/` 등)에 둡니다.
  - 결과를 다시 계산에 쓰지 않는 IO는 명령 안에서 해도 됩니다. 계산 결과를 파일에 한 번 쓰고 끝나는 경우가 그렇습니다. IO 결과에 따라 계산이 달라지거나 같은 IO를 여러 명령이 쓰면 모듈로 빼고 trait으로 받습니다.
- `#[cfg(target_os = ...)]` 코드는 `platform/`에만 둡니다.
- 이벤트 이름은 `events.rs`와 `types/ipc.ts`에 같은 이름의 상수로 둡니다. payload 타입은 `events.rs`에 정의하고 ts-rs로 내보냅니다.
- `tasks/`의 작업은 `setup`에서 `tauri::async_runtime::spawn`으로 띄웁니다. 계산은 로직 모듈에 맡기고, 작업은 값을 받아 넘기고 `emit`으로 알리기만 합니다.

### Rust와 TS 타입 맞추기

renderer와 주고받는 타입은 Rust에 정의하고 [ts-rs](https://github.com/Aleph-Alpha/ts-rs)(12 이상)로 TS 타입을 만듭니다. TS 쪽에 같은 타입을 손으로 쓰지 않습니다.

- 명령 인자와 반환값, 명령 에러, 이벤트 payload에 `#[derive(serde::Serialize, ts_rs::TS)]`와 `#[ts(export)]`를 붙입니다. 동작 없는 코드 생성이라 `domain/`의 타입에 붙여도 됩니다.
- ts-rs는 `cargo test`(`pnpm test:rust`)를 돌릴 때 `src/renderer/src/types/bindings/`에 파일을 씁니다. 이 위치와 `TS_RS_LARGE_INT = "number"`는 저장소 루트의 `.cargo/config.toml`에 있습니다. cargo는 실행한 디렉터리부터 위로 설정을 찾으므로 루트에서 도는 `pnpm test:rust`와 `src-tauri`에서 도는 `tauri dev` 모두 이 파일을 읽습니다.
- `u64`가 `number`가 되므로 2^53을 넘는 값은 문자열로 보냅니다.
- 생성된 파일은 커밋하고 직접 고치지 않습니다. Rust 타입을 바꾸면 `pnpm test:rust`로 다시 만들어 함께 커밋합니다. 이 폴더는 Prettier와 ESLint 검사에서 빠져 있습니다.
- ts-rs가 serde 속성(`rename_all`, `tag` 등)을 읽으므로 생성된 타입이 실제 JSON 모양과 같습니다.
- `Option<T>` 필드에는 `#[ts(optional)]`과 `#[serde(skip_serializing_if = "Option::is_none")]`를 함께 붙입니다. 그래야 TS 타입이 `T | null`이 아니라 `field?: T`가 되어 `undefined`만 쓰는 TS 규칙과 맞습니다.
- Rust에서 온 값(명령 반환값, 명령 에러, 이벤트 payload)은 생성된 타입으로 받고 다시 검사하지 않습니다. reject 값 중 명령 에러와 Tauri 오류를 가르는 일만 `api/`의 `call`이 합니다. `unknown`으로 받아 파싱하는 대상은 사용자 입력, 파일, 외부 네트워크처럼 Rust가 만들지 않은 값뿐입니다.

### 창 추가하기

창을 하나 추가할 때는 아래 다섯 곳을 함께 고칩니다. 창은 `tauri.conf.json`의 `app.windows`로 선언하고 OS별 창 동작은 `platform/`에 둡니다.

| 위치                        | 추가할 것                                       |
| --------------------------- | ----------------------------------------------- |
| `src/renderer/`             | `<창 이름>.html`, `src/windows/<창 이름>/`      |
| `vite.config.ts`            | `build.rollupOptions.input`에 HTML 등록         |
| `src-tauri/tauri.conf.json` | `app.windows`에 창 `label`과 `url`              |
| `src-tauri/src/window/`     | `<창 이름>.rs`. 그 창만의 Rust 코드가 있을 때만 |
| `src-tauri/capabilities/`   | `<창 이름>.json`. 그 창에 필요한 권한만 넣음    |

## 문서와 주석

- 한국어 문서(README, AGENTS.md 등), PR 본문, 커밋 메시지, 코드 주석은 `korean-tech-writing` 스킬을 따라 씁니다.
- 코드가 무엇을 하는지, 왜 그렇게 해야 하는지만 씁니다. 내부 논의 과정이나 의사결정 경위, 검토했다가 버린 방안은 쓰지 않습니다.
  - ✓ `// Windows에서 바이너리와 이름이 같으면 빌드가 깨지므로 _lib을 붙인다`
  - ✗ `// 처음엔 이름을 그대로 썼는데 논의 끝에 _lib을 붙이기로 했다`
- 명확하고 이해하기 쉽게 쓰되 장황하지 않게 씁니다. 코드만 읽어도 알 수 있는 내용은 주석으로 반복하지 않습니다.
- 함수, 컴포넌트, Hook, export하는 값과 타입처럼 선언에 다는 설명은 문서 주석으로 씁니다. TS는 `/** */`, Rust는 `///`입니다. 편집기에서 그 이름에 마우스를 올리면 설명이 보입니다. 함수 본문 안이나 코드 줄에 붙는 설명은 `//`로 씁니다.

  ```ts
  /** leva 스키마에서 기본값을 꺼낸다. folder()는 펼치고, button()은 값이 없으므로 건너뛴다. */
  function defaultsOf(schema: AnyRecord): AnyRecord {
    // 중첩된 folder()는 재귀로 펼친다
  }
  ```

  ```rust
  /// 창을 닫아도 앱은 종료하지 않고 창만 숨긴다. 종료는 Cmd+Q로 한다.
  pub fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {}
  ```

## 브랜치 전략과 릴리스

간소화한 Git Flow 위에서 release-please가 버전, 태그, CHANGELOG, GitHub Release를 만듭니다. 자세한 절차는 README의 "브랜치 전략과 릴리스"에 있습니다.

| 브랜치                     | 갈라지는 곳                           | PR base            | 머지 방식                                      | 나오는 버전       |
| -------------------------- | ------------------------------------- | ------------------ | ---------------------------------------------- | ----------------- |
| `feature/*`                | `develop`                             | `develop`          | squash                                         | `develop`의 alpha |
| `hotfix/*`                 | `main`                                | `main`             | squash                                         | `main`의 patch    |
| `fix/*`                    | `release/x.y.0`                       | 그 `release/x.y.0` | squash                                         | beta, rc          |
| `release/x.y.0`            | Promote 워크플로가 `develop`에서 만듦 | `main`             | merge commit                                   | 정식 버전         |
| Release PR, `back-merge/*` | 워크플로가 자동 생성                  | 각 브랜치          | Release PR은 squash, back-merge는 merge commit |                   |

- **PR 제목이 곧 릴리스 노트 한 줄입니다.** 저장소 설정상 squash 커밋은 제목이 PR 제목이고 본문이 비어 있어서, release-please는 PR 제목만 읽습니다. 사용자에게 바뀌는 점을 Conventional Commits 형식으로 씁니다. `pr-check.yml`이 `commitlint.config.mjs`로 검사합니다.
- **`release`, `promote` 타입은 쓰지 않습니다.** Promote 워크플로만 쓰는 타입입니다. 단계 전환(beta 시작, rc, stable)도 Promote 워크플로로만 합니다. 이 워크플로가 브랜치의 release-please 설정에 `release-as`를 넣고, 릴리스가 끝나면 `release-please.yml`이 지웁니다. `Release-As:` 커밋 footer는 back-merge로 다른 브랜치에 퍼지므로 쓰지 않습니다.
- **버전 파일은 브랜치마다 값이 다릅니다.** `package.json`의 `version`, `.release-please-manifest.json`, `release-please-config*.json`, `CHANGELOG.md`는 손으로 고치지 않습니다. back-merge 스크립트(`.github/scripts/back-merge.sh`)가 대상 브랜치의 값을 유지합니다.
- **hotfix는 브랜치 이름이 `hotfix/`로 시작해야 합니다.** `hotfix-back-merge.yml`은 head 브랜치 이름을 보고 `develop`과 진행 중인 `release/*`에 back-merge합니다.
- **`release/x.y.0` → `main` PR은 정식 버전 Release PR을 머지한 뒤에 엽니다.** 제목은 `Release x.y.0`처럼 Conventional Commits 형식이 아니게 씁니다. 그래야 main CHANGELOG에 중복 기록되지 않습니다.
- `develop`에는 바로 나가도 되는 코드만 머지합니다. 큰 기능은 사용자에게 보이는 연결(버튼, 메뉴, 라우트)을 마지막 PR로 분리합니다.

## SDD(OpenSpec)

기능은 [OpenSpec](https://github.com/Fission-AI/OpenSpec)으로 스펙을 먼저 쓰고 만듭니다. 입구는 `sdd-next` 스킬 하나입니다. 사람이 읽는 사용법은 README의 "스펙부터 쓰고 기능 만들기"에 있고, 여기에는 에이전트가 지킬 규칙만 적습니다.

- **상태는 스크립트가 정합니다**: `node scripts/sdd-state.mjs`가 변경 폴더를 보고 상태(proposal → specs → clarify → plan → approval → implement → verify → archive)를 판정합니다. `sdd-next`는 그 상태의 일 하나만 하고 멈춥니다. 상태를 추측하거나 두 상태 이상을 한 번에 진행하지 않습니다.
- **규칙은 한 곳에만 둡니다**: 단계별 규칙은 `openspec/schemas/namaste/schema.yaml`, 프로젝트 원칙과 산출물별 규칙은 `openspec/config.yaml`에 있습니다. 스킬과 문서에는 옮겨 적지 않습니다. 규칙을 바꿀 때는 이 두 파일을 고치고 `pnpm exec openspec schema validate namaste`와 `pnpm sdd:check`를 돌립니다.
- **clarify는 다시 할 수 있습니다**: `sdd-next clarify`로 proposal 이후 archive 전 어느 상태에서든 clarify를 한 라운드 더 합니다.
- **앞 단계로 돌아갈 수 있습니다**: 앞 단계 산출물을 고쳐야 하면 멈추고 사람에게 묻습니다. 고치면 뒤 단계에 적힌 해시(`spec-hash:`, `x-approved:`, `plan-hash:`)가 달라져 상태 판정이 그 다음 단계부터 다시 진행합니다. 절차는 `openspec/config.yaml`의 `context`에 있습니다.
- **승인은 사람만 합니다**: `.openspec.yaml`의 `x-approved:`는 사람이 명시적으로 승인했을 때만 `node scripts/sdd-state.mjs <change> --hash approval` 값으로 씁니다. 승인 전에는 구현하지 않습니다.
- **테스트는 구현을 모르는 쪽이 먼저 씁니다**: implement 단계에서 `test-writer` 서브에이전트가 delta 스펙과 `plan.md`의 설계만 보고 테스트를 씁니다. 구현하는 쪽은 테스트 파일을 고치지 않고, 이의는 `test-disputes.md`에 적습니다.
- **요구사항 ID**: 기능 요구사항(Functional Requirement)은 `FR-<도메인>-NNN`, 성공 기준(Success Criteria)은 `SC-<도메인>-NNN`입니다. 도메인 코드는 영어 대문자 2~6자이고 capability 경로는 그 소문자입니다. 한 번 쓴 ID는 지워진 뒤에도 다시 쓰지 않습니다.
- **스펙이 쌓이는 곳**:

  | 무엇                            | 위치                                                    | 누가 바꾸나 |
  | ------------------------------- | ------------------------------------------------------- | ----------- |
  | 구현된 요구사항 전체(기준 스펙) | `openspec/specs/<도메인>/spec.md`, 도메인마다 파일 하나 | archive만   |
  | 진행 중인 변경                  | `openspec/changes/<이름>/`                              | `sdd-next`  |
  | 끝난 변경의 기록                | `openspec/changes/archive/<날짜>-<이름>/`               | archive만   |
  | 단계 규칙                       | `openspec/schemas/namaste/`, `openspec/config.yaml`     | 사람이 PR로 |

- **archive는 머지 전에**: 기능 PR을 머지하기 전에 같은 브랜치에서 archive까지 끝냅니다. 그래야 `develop`의 기준 스펙이 `develop`의 코드와 같습니다. `openspec/specs/`는 손으로 고치지 않습니다.
- OpenSpec CLI는 `devDependencies`에 고정한 버전을 `pnpm exec openspec`으로 씁니다. 사용 통계를 보내지 않도록 `OPENSPEC_TELEMETRY=0`을 붙입니다. OpenSpec이 만드는 기본 스킬과 명령(`openspec-*`, `/opsx:*`)은 설치하지 않습니다. `openspec init`이나 `openspec update`를 다시 실행할 때는 `--tools none`을 씁니다.

## 스킬

반복 작업의 절차는 `.agents/skills/<이름>/SKILL.md`에 있습니다. 해당 작업을 할 때는 먼저 그 파일을 읽고 따르세요. 스킬을 불러오는 기능이 있는 에이전트는 그 기능으로 불러와도 됩니다. SKILL.md 안의 경로는 스킬 폴더 밖의 파일이면 저장소 루트 기준이고, 같은 스킬의 `references/` 같은 파일이면 그 스킬 폴더 기준입니다.

**`src/renderer`나 `src-tauri`의 파일, 또는 빌드와 앱 설정을 바꾸게 되는 작업은 `code-writer`부터 부릅니다.** 질문이나 리뷰로 시작했더라도 파일을 고치기로 한 시점에 부릅니다. 아래 표의 다른 코드 스킬은 `code-writer`가 필요한 것만 골라 부릅니다. 같은 일을 하는 전역 스킬(예: 커밋, 함수형 리뷰)이 보여도 이 표의 저장소 스킬을 씁니다.

| 스킬                          | 쓰는 때                                                                                                                                                                                                                                |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `code-writer`                 | 코드를 쓰거나 고칠 때 가장 먼저. 건드리는 층에 맞춰 아래 코드 스킬 중 필요한 것을 골라 부르고, 모르는 API는 공식 문서로 확인합니다                                                                                                     |
| `commit`                      | 커밋. 커밋할 때 commitlint를 돌리지 않으므로 스킬의 형식 규칙(설명 첫 글자를 대문자로 쓰지 않기 등)을 직접 지킵니다                                                                                                                    |
| `pr`                          | PR 생성. base 판단, 릴리스 노트용 제목, `.github/PULL_REQUEST_TEMPLATE.md` 채우기                                                                                                                                                      |
| `korean-tech-writing`         | 한국어 문서, PR 본문, 커밋 메시지, 주석                                                                                                                                                                                                |
| `humanizer`                   | 한국어 글의 AI 말투 점검. `korean-tech-writing`이 긴 문서에서 부릅니다                                                                                                                                                                 |
| `threejs-*`                   | `code-writer`가 3D 작업에서 부릅니다. 코드를 바꾸지 않는 질문에는 바로 씁니다. 이 앱은 R3F를 쓰므로 `threejs-react-three-fiber`부터 봅니다. 아래 CSP 제약을 함께 지킵니다                                                              |
| `zustand-state-management`    | `code-writer`가 store 작업에서 부릅니다. 코드를 바꾸지 않는 질문에는 바로 씁니다. R3F에서 쓸 때는 아래 규칙을 함께 지킵니다                                                                                                            |
| `vercel-react-best-practices` | `code-writer`가 React 작업에서 부릅니다. 코드를 바꾸지 않는 리뷰와 질문에는 바로 씁니다. Next.js, RSC, SSR 규칙은 이 앱에 해당하지 않습니다                                                                                            |
| `fp-apply`                    | `code-writer`가 로직 작업에서 부릅니다. 코드를 바꾸지 않는 설계 질문과 함수형 리뷰에는 바로 씁니다. 계산 규칙은 순수 함수로 두고, 사이드 이펙트는 이벤트 핸들러, `Effect`, store action, `useFrame`, `api/`, Rust 명령 쪽으로 나눕니다 |
| `sdd-next`                    | 기능을 스펙부터 만들 때. 새 기능을 시작하거나 진행 중인 `openspec/changes/` 작업을 이어 갈 때 부릅니다. 위 "SDD(OpenSpec)" 규칙을 함께 지킵니다                                                                                        |

- 스킬 원본은 `.agents/skills/` 한 곳에만 둡니다. 에이전트별 스킬 폴더(예: `.claude/skills/`)에는 원본을 가리키는 스킬별 심링크만 둡니다. 스킬을 추가하거나 고칠 때도 `.agents/skills/`에서 작업합니다.
- `.agents`는 `.prettierignore`와 ESLint `ignores`에 들어 있습니다. 외부에서 가져온 스킬을 원본 그대로 두기 위해서입니다.
- `threejs-*` 13개는 [CloudAI-X/threejs-skills](https://github.com/CloudAI-X/threejs-skills)(MIT)를 그대로 가져온 것입니다. 갱신할 때도 원본을 덮어쓰고 직접 고치지 않습니다.
- **3D 작업의 CSP 제약**: CSP가 외부 주소를 막으므로 에셋과 디코더는 앱 안(`'self'`)에서만 불러옵니다. drei `Environment`의 `preset`(외부 HDR)은 쓰지 말고 `files`에 앱 안의 HDR 파일을 넘깁니다. `useGLTF`의 기본 Draco 디코더도 gstatic에서 받아 오므로, Draco 모델을 쓸 때는 디코더를 앱에 넣고 `useGLTF.setDecoderPath`로 그 경로를 가리킵니다.
- `zustand-state-management`는 [mindrally/skills](https://github.com/mindrally/skills)(Apache-2.0)를 그대로 가져온 것입니다.
- `vercel-react-best-practices`는 [vercel-labs/agent-skills](https://github.com/vercel-labs/agent-skills)(MIT)의 `skills/react-best-practices`를 그대로 가져온 것입니다.
- `fp-apply`는 직접 쓴 스킬입니다. 기술별 방법론만 담고, 이 저장소의 폴더나 빌드 설정, 다른 스킬은 다루지 않습니다. 이 저장소에 적용하는 위치와 제약은 이 파일("구조", "폴더 구조")에 적습니다. 그래서 저장소 구조가 바뀌어도 스킬은 고치지 않습니다.
- `code-writer`는 직접 쓴 지휘 스킬입니다. 절차와 라우팅만 담고, 규칙은 이 파일과 각 스킬의 위치를 가리키기만 합니다. 스킬을 추가하거나 빼면 `code-writer`의 라우팅 표도 함께 고칩니다.
- **스킬끼리 다를 때**: 이 파일, `fp-apply`, 외부 스킬 순서로 따릅니다. 외부 스킬은 원본 그대로 두므로 이 앱과 맞지 않는 내용이 있습니다.
  - `zustand-state-management`가 권하는 immer 미들웨어는 쓰지 않습니다. 중첩 갱신은 store 밖의 순수 함수에서 immer `produce`로 합니다.
  - `fp-apply`가 IO 경계에서 값을 `unknown`으로 받아 파싱하라고 하는 부분(`typescript.md`의 "외부 입력은 경계에서 파싱", "사이드 이펙트와 비동기"의 `as` 설명)은 Rust에서 온 값에는 적용하지 않습니다. 위 "Rust와 TS 타입 맞추기"와 기능 추가 2단계를 따릅니다.
  - `fp-apply`의 "바깥에서 온 `null`은 `undefined`로 바꾼다"는 Rust 타입에 `#[ts(optional)]`을 붙여 처음부터 `undefined`가 되게 합니다. 바꾸는 코드를 따로 두지 않습니다.
  - immer의 `Map`, `Set` draft(`enableMapSet()`)는 쓰지 않습니다. 상태에는 배열과 객체를 씁니다.
  - `vercel-react-best-practices`가 권하는 `toSorted`, `requestIdleCallback`, CSS `content-visibility`는 macOS 빌드 타깃에서 동작하지 않습니다(위 "구조"의 웹뷰 항목).
- **R3F에서 zustand 쓰기**: 3D 쪽과 Canvas 밖 HUD가 같이 쓰는 상태는 `stores/`의 store로 주고받습니다. `useFrame` 안에서는 selector Hook 대신 `useXStore.getState()`로 값을 읽습니다. 매 프레임 바뀌는 값(위치, 속도 등)은 store에 넣어 리렌더링을 일으키지 말고 ref로 다룹니다.
