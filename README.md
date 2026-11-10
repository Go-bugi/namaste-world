# Namaste World

Tauri 2 + React + TypeScript + Vite 데스크톱 앱.

## 요구 사항

- Node.js **24.21.0+** (`.nvmrc` 참고)
- pnpm **10** — 프로젝트에 버전이 고정되어 있으므로 corepack 사용을 권장합니다.
- Rust **stable** ([rustup](https://rustup.rs/)으로 설치, `rustup update stable`로 갱신)
- OS별 준비물: [Tauri 사전 요구 사항](https://v2.tauri.app/start/prerequisites/)
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft C++ Build Tools, WebView2(Windows 10 이상은 기본 설치)

```bash
corepack enable
```

## 시작하기

```bash
pnpm install
pnpm dev
```

첫 실행은 Rust 의존성을 컴파일하느라 몇 분 걸립니다. 그다음부터는 바뀐 부분만 다시 빌드합니다.

## 스크립트

| 스크립트                         | 설명                                                        |
| -------------------------------- | ----------------------------------------------------------- |
| `pnpm dev`                       | 개발 모드 (renderer HMR, Rust 변경 시 다시 빌드하고 재시작) |
| `pnpm build`                     | 타입체크 후 renderer 빌드 (`out/renderer`)                  |
| `pnpm build:mac`                 | macOS dmg 패키징 (지금 Mac의 아키텍처)                      |
| `pnpm build:win`                 | Windows nsis 설치 파일 (x64)                                |
| `pnpm build:win:arm64`           | Windows ARM 설치 파일                                       |
| `pnpm build:unpack`              | 설치 파일 없이 실행 파일만 빌드 (빠른 확인용)               |
| `pnpm lint` / `format`           | ESLint / Prettier                                           |
| `pnpm lint:rust` / `format:rust` | Clippy / rustfmt                                            |
| `pnpm typecheck`                 | `vite.config.ts`(node)와 renderer(web) 각각 타입체크        |

결과물은 `src-tauri/target/release/bundle/`(다른 target을 지정하면 `src-tauri/target/<target>/release/bundle/`)에 생성됩니다.

## 구조

```
src/
  renderer/          React 앱
    src/api/         Rust 명령을 invoke로 감싼 함수 모음
src-tauri/
  src/lib.rs         앱 진입점. Builder 조립
  src/commands/      Rust 명령(#[tauri::command])
  capabilities/      renderer에 허용할 권한
  icons/             앱 아이콘 (pnpm tauri icon <png>로 다시 생성)
  tauri.conf.json    창, 보안(CSP), 번들 설정
vite.config.ts
```

- renderer는 OS에 직접 접근할 수 없습니다. 필요한 기능은 `src-tauri/src/commands/`에 명령으로 만들고 `src/renderer/src/api/`의 `api`에 추가하세요. 플러그인을 쓰면 그 창의 capability 파일(`capabilities/main.json` 등)에 권한을 추가해야 합니다.
- 앱 버전은 `package.json`의 `version`을 그대로 씁니다. `src-tauri/Cargo.toml`의 버전은 `0.0.0`으로 고정되어 있습니다.

## 디버깅

- renderer: 개발 모드에서 창을 우클릭해 **Inspect**를 고르거나, macOS는 `Cmd+Option+I`, Windows는 `Ctrl+Shift+I`로 DevTools를 엽니다.
- Rust: `println!` 출력은 `pnpm dev`를 실행한 터미널에 나옵니다. 중단점이 필요하면 VS Code에 CodeLLDB 확장을 설치해 붙입니다.

## 포맷팅 / 커밋 규칙

- 저장 시 Prettier 포맷 + ESLint 자동 수정 (`.vscode/settings.json`)
  — 추천 확장(Prettier, ESLint, EditorConfig, Tauri, rust-analyzer)을 설치하세요.
- 커밋 시 husky + lint-staged가 변경된 파일에 ESLint, Prettier, rustfmt를 적용합니다.
- 줄바꿈은 OS와 무관하게 **LF**로 통일됩니다 (`.gitattributes`, `.editorconfig`, Prettier `endOfLine`).

### Windows 개발 시

- 클론하기 전에 아래 설정을 권장합니다. `.gitattributes`가 LF를 강제하지만, 전역 설정까지 맞춰두면 안전합니다.

  ```bash
  git config --global core.autocrlf false
  ```

- husky 훅은 Git for Windows에 포함된 sh로 실행됩니다. GUI Git 클라이언트를 쓴다면 `pnpm`이 PATH에 있어야 합니다.

## 브랜치 전략과 릴리스

간소화한 Git Flow와 [release-please](https://github.com/googleapis/release-please)를 사용합니다.
버전, 태그, CHANGELOG, GitHub Release는 release-please가 자동으로 관리합니다.

### 브랜치

| 브랜치          | 만드는 곳                | 머지하는 곳           | 나오는 버전                  |
| --------------- | ------------------------ | --------------------- | ---------------------------- |
| `main`          | -                        | -                     | 정식 버전, hotfix (`v1.3.1`) |
| `develop`       | `main`                   | -                     | alpha (`v1.4.0-alpha.N`)     |
| `feature/*`     | `develop`                | `develop` (squash)    | -                            |
| `release/x.y.0` | Promote 버튼 (`develop`) | `main` (merge commit) | beta → rc → 정식 (`v1.3.0`)  |
| `hotfix/*`      | `main`                   | `main` (squash)       | -                            |

- `develop`에는 **지금 바로 나가도 되는 코드만** 머지합니다.
  - 작은 기능은 완성된 뒤에 머지합니다.
  - 큰 기능은 사용자에게 보이는 연결(버튼, 메뉴, 라우트)을 마지막 PR로 분리합니다.
- `release/*`에는 버그 수정만 들어갑니다.

### 브랜치별 release-please 설정

`.github/workflows/release-please.yml`이 push된 브랜치에 따라 설정 파일을 골라 실행합니다.

| 브랜치      | 설정 파일                            | 버전 규칙                               |
| ----------- | ------------------------------------ | --------------------------------------- |
| `main`      | `release-please-config.json`         | 기본 (fix → patch, feat → minor)        |
| `develop`   | `release-please-config.develop.json` | prerelease, `alpha.N` 번호 증가         |
| `release/*` | `release-please-config.release.json` | prerelease, `beta.N` / `rc.N` 번호 증가 |

- 현재 버전은 브랜치마다 `.release-please-manifest.json`에 기록되며, 브랜치마다 값이 다릅니다.
- 1.0 전까지는 breaking change가 있어도 minor만 올라갑니다 (`bump-minor-pre-major`).

### 릴리스 방법

**Release PR**: 브랜치에 `feat`, `fix` 등이 머지되면 release-please가 `chore(develop): release 1.4.0-alpha.3` 같은 Release PR을 만들거나 갱신합니다. **이 PR을 머지하는 순간** 태그, GitHub Release, mac / Windows 빌드 업로드가 진행됩니다.

**단계 전환**: Actions → **Promote** 워크플로를 실행합니다.

| 단계            | 하는 일                                                                      |
| --------------- | ---------------------------------------------------------------------------- |
| `start-release` | `develop`에서 `release/x.y.0` 생성 → beta.1. `develop`은 다음 버전 alpha.1로 |
| `rc`            | `release/x.y.0`을 rc.1로                                                     |
| `stable`        | `release/x.y.0`을 정식 버전으로                                              |

**출시 순서**

1. Promote `start-release` (버전 `1.3.0`) → beta.1 Release PR 머지
2. 버그 수정은 `release/1.3.0`으로 PR (squash) → beta.N Release PR 머지
3. Promote `rc` → rc.1 Release PR 머지
4. Promote `stable` → `1.3.0` Release PR 머지
5. `release/1.3.0` → `main` PR을 **merge commit**으로 머지 (제목은 `Release 1.3.0`처럼, Conventional Commits 형식 아님)
6. 자동으로 만들어진 `develop` back-merge PR을 **merge commit**으로 머지

태그는 Release PR을 머지할 때 붙습니다. 속이 빈 원은 다른 브랜치를 합친 머지 커밋입니다.

```mermaid
%%{init: {'gitGraph': {'mainBranchName': 'main', 'showCommitLabel': false}} }%%
gitGraph
    commit tag: "v1.2.0"
    branch develop order: 3
    commit
    branch feature/x order: 4
    commit
    commit
    checkout develop
    merge feature/x
    commit tag: "v1.3.0-alpha.1"
    branch release/1.3.0 order: 1
    commit tag: "v1.3.0-beta.1"
    checkout develop
    commit
    commit tag: "v1.4.0-alpha.1"
    checkout release/1.3.0
    branch fix/x order: 2
    commit
    checkout release/1.3.0
    merge fix/x
    commit tag: "v1.3.0-beta.2"
    commit
    commit tag: "v1.3.0-rc.1"
    commit
    commit tag: "v1.3.0"
    checkout main
    merge release/1.3.0
    checkout develop
    merge release/1.3.0
```

**hotfix**

1. `main`에서 `hotfix/x.y.z` 브랜치를 만들어 수정
2. `main`으로 PR (`fix: ...`, **squash**) → `main`의 Release PR 머지
3. `develop`과 진행 중인 `release/*`에는 자동으로 반영됩니다.

```mermaid
%%{init: {'gitGraph': {'mainBranchName': 'main', 'showCommitLabel': false}} }%%
gitGraph
    commit
    branch develop order: 2
    commit
    branch release/1.4.0 order: 3
    commit tag: "v1.4.0-beta.1"
    checkout main
    commit tag: "v1.3.0"
    branch hotfix/1.3.1 order: 1
    commit
    checkout main
    merge hotfix/1.3.1
    commit tag: "v1.3.1"
    checkout develop
    merge main
    checkout release/1.4.0
    merge main
```

### 규칙

| 규칙                                                                   | 이유                                                                        |
| ---------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| PR 제목은 Conventional Commits (`feat:`, `fix:` 등). `pr-check`가 검사 | squash 시 PR 제목이 커밋 메시지가 되고, release-please가 이걸로 버전을 정함 |
| merge commit은 `release → main`과 `back-merge/*` PR만, 나머지는 squash | hotfix를 merge commit으로 하면 CHANGELOG에 중복 기록됨                      |
| 단계 전환은 Promote 버튼으로만                                         | `release-as` 설정을 넣고 릴리스 후 자동으로 지움                            |
| `release:`, `promote:` 타입은 사람이 쓰지 않음                         | Promote 워크플로 전용 (CHANGELOG의 Release 섹션)                            |

### 자동 back-merge

`.github/scripts/back-merge.sh`를 사용하며, 대상 브랜치의 버전 파일(`package.json`의 version, manifest, release-please 설정, CHANGELOG)은 항상 그대로 유지합니다.

- **hotfix** (`hotfix-back-merge.yml`): `hotfix/*` PR 또는 `main`의 Release PR이 머지되면 `develop`과 진행 중인 `release/*`에 반영합니다. 충돌이 없으면 바로 push하고, 충돌이 있거나 push가 거부되면 PR을 만듭니다.
- **release 출시** (`release-back-merge.yml`): `release/*`가 `main`에 머지되면 `develop`으로 back-merge PR을 만들고 `release/*` 브랜치를 삭제합니다.

> GitHub Actions 기본 토큰(`GITHUB_TOKEN`)으로 만든 PR에서는 다른 워크플로가 실행되지 않습니다. 그래서 Release PR과 back-merge PR에는 `pr-check`가 붙지 않습니다. 브랜치 보호에 "CI 통과 필수"를 걸려면 GitHub App 토큰으로 바꿔야 합니다.

## 빌드 환경

| 대상    | macOS에서 | Windows에서            |
| ------- | --------- | ---------------------- |
| macOS   | ✅        | ❌ (Apple 정책상 불가) |
| Windows | ❌        | ✅                     |

Tauri는 각 OS에서 빌드하는 것을 기준으로 합니다. Windows 설치 파일은 Windows에서, macOS dmg는 Mac에서 만듭니다. Mac에서 Intel용을 만들려면 `rustup target add x86_64-apple-darwin` 후 `pnpm tauri build --target x86_64-apple-darwin --bundles dmg`를 실행합니다.

release-please가 릴리스를 만들면 `.github/workflows/release-please.yml`이 macOS(arm64, x64) / Windows(x64) 러너에서 각각 빌드해 `namaste-world-<version>-mac-arm64.dmg`, `namaste-world-<version>-win-x64-setup.exe` 같은 이름으로 GitHub Release에 업로드합니다. PR에서는 `pr-check.yml`이 포맷, 린트, 타입체크, renderer 빌드와 Rust 검사(rustfmt, clippy)만 확인합니다.

### 코드 서명

현재는 서명 없이 빌드됩니다(macOS는 ad-hoc 서명). 배포할 때는 [Tauri 서명 문서](https://v2.tauri.app/distribute/sign/macos/)를 따라 secrets를 설정하세요.

- macOS: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, 공증용 `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`
- Windows: `tauri.conf.json`의 `bundle.windows`에 인증서 지문(`certificateThumbprint`)이나 `signCommand`를 설정
