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
pnpm build            # typecheck 후 renderer만 out/renderer에 빌드
pnpm build:unpack     # 설치 파일 없이 앱 실행 파일만 빌드 (src-tauri/target/release/)
pnpm build:mac        # dmg 패키징. build:win은 nsis 설치 파일, build:win:arm64는 Windows ARM용 nsis 설치 파일
```

- 테스트 러너는 아직 없습니다. PR CI(`pr-check.yml`)는 `format:check`, `lint`, `build`, `format:check:rust`, `lint:rust`를 돌립니다.
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
- **renderer는 OS에 직접 접근할 수 없습니다.** Rust 명령이나 플러그인을 거쳐야 하고, 쓸 수 있는 기능은 `src-tauri/capabilities/*.json`에 선언한 권한으로 제한됩니다. 기능을 추가할 때는 아래를 함께 고칩니다.
  1. `src-tauri/src/lib.rs`에 `#[tauri::command]` 함수를 만들고 `generate_handler!`에 등록합니다.
  2. `src/renderer/src/api.ts`의 `api`에 그 명령을 `invoke`로 감싸는 함수를 추가합니다. 컴포넌트에서 `invoke`를 직접 부르지 않습니다.
  3. 플러그인이나 코어 API를 쓰면 `src-tauri/capabilities/default.json`의 `permissions`에 필요한 권한만 추가합니다.
- **CSP는 `tauri.conf.json`의 `app.security.csp`에서 설정합니다.** `index.html`에 `<meta>`로 넣지 않습니다. IPC가 `ipc:`/`http://ipc.localhost`를 쓰므로 `connect-src`에서 이 둘을 빼면 안 됩니다.
- **tsconfig는 둘로 나뉩니다.** `tsconfig.node.json`은 `vite.config.ts`를, `tsconfig.web.json`은 renderer를 다룹니다. renderer에서는 `@renderer/*` 별칭으로 `src/renderer/src`를 가리킵니다.
- 앱 ID는 `tauri.conf.json`의 `identifier`(`com.aibuddy.app`)입니다. Windows 알림과 작업 표시줄 그룹에도 이 값이 쓰입니다.
- **앱 버전은 `package.json` 하나에서 옵니다.** `tauri.conf.json`의 `version`이 `"../package.json"`을 가리킵니다. `Cargo.toml`의 `version`은 `0.0.0`으로 고정하고 올리지 않습니다. 올리면 브랜치마다 값이 달라져 back-merge 때 `Cargo.toml`, `Cargo.lock`이 충돌합니다.
- 렌더러 의존성은 Vite가 번들하므로 지금처럼 `devDependencies`에 추가합니다. Rust 의존성은 `src-tauri/Cargo.toml`에 추가하고 `Cargo.lock`도 커밋합니다(CI가 `--locked`로 검사).
- 기획 문서는 `design/`에 둡니다. 기능별 MVP 결정은 `design/mvp-feature-decisions.md`에 있고, 아직 정하지 않은 항목이 남아 있으니 기능을 만들기 전에 읽으세요.

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

## 스킬

반복 작업의 절차는 `.agents/skills/<이름>/SKILL.md`에 있습니다. 해당 작업을 할 때는 먼저 그 파일을 읽고 따르세요. 스킬을 불러오는 기능이 있는 에이전트는 그 기능으로 불러와도 됩니다. SKILL.md 안에서 다른 파일을 가리키는 경로는 저장소 루트 기준입니다.

| 스킬                       | 쓰는 때                                                                                                               |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `commit`                   | 커밋. 커밋할 때 commitlint를 돌리지 않으므로 스킬의 형식 규칙(설명 첫 글자를 대문자로 쓰지 않기 등)을 직접 지킵니다   |
| `pr`                       | PR 생성. base 판단, 릴리스 노트용 제목, `.github/PULL_REQUEST_TEMPLATE.md` 채우기                                     |
| `korean-tech-writing`      | 한국어 문서, PR 본문, 커밋 메시지, 주석                                                                               |
| `humanizer`                | 한국어 글의 AI 말투 점검. `korean-tech-writing`이 긴 문서에서 부릅니다                                                |
| `threejs-*`                | 3D 씬과 R3F 컴포넌트 작업. 이 앱은 R3F를 쓰므로 `threejs-react-three-fiber`부터 봅니다. 아래 CSP 제약을 함께 지킵니다 |
| `zustand-state-management` | zustand store 설계와 사용. R3F에서 쓸 때는 아래 규칙을 함께 지킵니다                                                  |

- 스킬 원본은 `.agents/skills/` 한 곳에만 둡니다. 에이전트별 스킬 폴더(예: `.claude/skills/`)에는 원본을 가리키는 스킬별 심링크만 둡니다. 스킬을 추가하거나 고칠 때도 `.agents/skills/`에서 작업합니다.
- `.agents`는 `.prettierignore`와 ESLint `ignores`에 들어 있습니다. 외부에서 가져온 스킬을 원본 그대로 두기 위해서입니다.
- `threejs-*` 13개는 [CloudAI-X/threejs-skills](https://github.com/CloudAI-X/threejs-skills)(MIT)를 그대로 가져온 것입니다. 갱신할 때도 원본을 덮어쓰고 직접 고치지 않습니다.
- **3D 작업의 CSP 제약**: CSP가 외부 주소를 막으므로 에셋과 디코더는 앱 안(`'self'`)에서만 불러옵니다. drei `Environment`의 `preset`(외부 HDR)은 쓰지 말고 `files`에 앱 안의 HDR 파일을 넘깁니다. `useGLTF`의 기본 Draco 디코더도 gstatic에서 받아 오므로, Draco 모델을 쓸 때는 디코더를 앱에 넣고 `useGLTF.setDecoderPath`로 그 경로를 가리킵니다.
- `zustand-state-management`는 [mindrally/skills](https://github.com/mindrally/skills)(Apache-2.0)를 그대로 가져온 것입니다.
- **R3F에서 zustand 쓰기**: 3D 쪽과 Canvas 밖 HUD는 store로 상태를 나눕니다. `useFrame` 안에서는 훅 selector 대신 `useXStore.getState()`로 값을 읽습니다. 매 프레임 바뀌는 값(위치, 속도 등)은 store에 넣어 리렌더링을 일으키지 말고 ref로 다룹니다.
