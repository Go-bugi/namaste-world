---
name: commit
description: 이 저장소의 변경 사항을 Conventional Commits 형식으로 커밋합니다. diff를 읽고 type과 scope를 고르고, PR 제목 검사와 같은 commitlint 형식 규칙(설명 첫 글자 대문자 금지 등)을 지켜 한국어 명사구로 메시지를 쓰고 커밋합니다. release-please 브랜치 전략(develop, release/*, main)과 예약 타입(release, promote)을 고려하고, 서로 다른 변경이 섞여 있거나 실수로 staging된 파일이 보이면 나눠서 커밋할지 먼저 확인합니다. 사용자가 "커밋해 줘", "커밋 메시지 써 줘", "이거 나눠서 커밋해", "변경 사항 정리해서 올려", "/commit"이라고 하거나, 작업을 마치고 커밋을 남겨 달라고 할 때 반드시 이 스킬을 사용하세요. PR 제목이나 PR 생성은 이 스킬의 범위가 아닙니다.
---

# commit

## 이 스킬이 존재하는 이유

이 저장소는 release-please로 버전과 CHANGELOG를 만듭니다. PR은 squash merge하므로 release-please가 실제로 읽는 건 **PR 제목**이고, 브랜치 안의 개별 커밋은 검사하지도, 릴리스 노트에 쓰지도 않습니다(`commitlint.config.mjs` 주석 참고).

그래도 개별 커밋을 Conventional Commits로 남기는 이유는 두 가지입니다. 나중에 PR 제목을 정할 때 커밋 목록이 근거가 되고, `git log`만 봐도 이 브랜치가 기능인지 수정인지 바로 보입니다. 그래서 이 스킬은 **형식은 엄격하게, 진행은 가볍게** 합니다. 커밋할 때 commitlint를 돌리지 않는 대신 3단계의 형식 규칙을 메시지를 쓰면서 직접 지키고, 판단이 필요한 지점에서만 사용자에게 묻습니다.

## 진행 순서

### 1. 상태와 브랜치 확인

```bash
git branch --show-current
git status --porcelain
git diff --staged
git diff
```

새 파일(`??`)은 diff에 안 나오므로 필요한 파일은 직접 열어 봅니다.

**브랜치에 따라 확인할 것**

- `main`, `develop`, `release/*`에 있으면, 커밋 전에 작업 브랜치를 만들지 묻습니다. 이 브랜치들은 push되는 순간 release-please가 돌기 때문에, 원래는 작업 브랜치에서 커밋하고 PR로 머지합니다. diff를 보고 브랜치 이름을 하나 제안합니다. 접두사는 지금 브랜치에 따라 정해집니다. PR 스킬이 이 접두사로 PR base를 확인하기 때문입니다.
  - `develop`에서는 `feature/...` (예: `feature/chat-search`)
  - `main`에서는 `hotfix/...` (예: `hotfix/window-crash`)
  - `release/*`에서는 `fix/...` (예: `fix/dark-mode-button-hover`)

  사용자가 그대로 진행하라고 하면 지금 브랜치에 커밋합니다.
  - 예외: 커밋이 하나도 없는 초기 저장소(`git rev-parse HEAD`가 실패)는 묻지 않고 진행합니다.
- `release/*`에서 `feat` 커밋을 만들게 되면 한 번 확인합니다. release 브랜치는 출시 전 안정화 단계라 보통 `fix`만 들어갑니다.
- 브랜치 이름으로 type을 정하지 않습니다. `hotfix/` 브랜치라도 diff가 문서 수정이면 `docs`입니다.

### 2. 커밋 단위 정하기

변경을 논리 단위로 묶습니다. 한 커밋에는 한 가지 이유만 담습니다.

**바로 커밋하는 경우**

- staging된 변경이 한 가지 논리 단위이고, staging되지 않은 변경이 없을 때
- 아무것도 staging되지 않았고, 전체 변경이 한 가지 논리 단위일 때 (관련 파일만 골라 `git add`)

**먼저 확인하는 경우.** 계획을 보여 주고 답을 받은 뒤 진행합니다.

- 서로 무관한 변경이 섞여 있을 때. 어떻게 나눌지 제안합니다.
  ```
  커밋을 두 개로 나누려고 합니다.
  1. feat(chat): 대화 기록 검색 추가  ← src/renderer/src/..., src/preload/...
  2. chore: eslint 규칙 정리          ← eslint.config.mjs
  ```
- 이미 staging된 것 중에 실수로 보이는 파일이 있을 때. staging은 사용자가 고른 것이지만 잘못 올렸을 수도 있으니, 그대로 믿지 말고 내용을 봅니다.
  - `.env*`, 키·인증서·토큰이 든 파일 (이건 확인 없이도 절대 커밋하지 않고 알립니다)
  - `out/`, `release/`, `node_modules/` 같은 빌드 산출물
  - 디버그용 `console.log`, 임시 파일, 변경 의도와 무관한 `pnpm-lock.yaml` 변경
  - 나머지와 관계없는 파일
- staging된 것 외에 staging되지 않은 변경도 있을 때. 같이 넣을지, 남겨 둘지 묻습니다.

나눌 때는 `git add <파일>`로 파일 단위로 고릅니다. 한 파일 안에서 나눠야 하면 `git add -p`는 대화형이라 쓸 수 없으니, 사용자에게 알리고 파일 단위로 나눌지 묻습니다.

### 3. 메시지 쓰기

```
type(scope): 설명

본문 (필요할 때만)

footer (필요할 때만)
```

**type**은 `commitlint.config.mjs`의 `type-enum`이 기준입니다. 설정이 바뀌었을 수 있으니 처음 한 번 열어서 확인합니다. release-please에서의 의미는 이렇습니다.

| type       | 언제                                      | release-please            |
| ---------- | ----------------------------------------- | ------------------------- |
| `feat`     | 새 기능, **화면 변경**                    | 릴리스 노트에 표시, minor |
| `fix`      | 버그 수정, **화면 버그 수정**             | 릴리스 노트에 표시, patch |
| `perf`     | 성능 개선                                 | 릴리스 노트에 표시, patch |
| `revert`   | 되돌리기                                  | 릴리스 노트에 표시, patch |
| `refactor` | 동작이 같은 구조 변경                     | 숨김                      |
| `style`    | 코드 포맷만 변경                          | 숨김                      |
| `docs`     | 문서만                                    | 숨김                      |
| `test`     | 테스트만                                  | 숨김                      |
| `build`    | 빌드 설정, 의존성, `electron-builder.yml` | 숨김                      |
| `ci`       | `.github/` 워크플로, 스크립트             | 숨김                      |
| `chore`    | 그 밖의 유지 보수                         | 숨김                      |

헷갈리기 쉬운 것들입니다.

- `style`은 **코드 포맷**입니다. 버튼 색이나 레이아웃처럼 사용자가 보는 화면이 바뀌면 `feat`나 `fix`입니다.
- `release`와 `promote`는 commitlint상 허용되지만 **Promote 워크플로 전용**입니다. 이 타입으로 커밋하면 develop이나 release 브랜치의 단계 전환 기록과 섞입니다. 사용자가 요청해도 쓰지 말고, GitHub Actions의 Promote 워크플로를 쓰라고 안내합니다.
- 버전을 바꾸려는 `Release-As:` footer도 쓰지 않습니다. back-merge로 다른 브랜치에 퍼져 버전을 덮어씁니다(`promote.yml` 주석 참고).

**scope**는 선택입니다. 변경이 한 영역에 모여 있을 때 짧은 영어 소문자로 붙입니다(`chat`, `ui`, `main`, `preload`, `release`). 정해진 목록은 없으니, `git log --oneline -20`에 이미 쓰인 scope가 있으면 그걸 우선합니다. 여러 영역에 걸치면 생략합니다.

**설명**은 한국어 명사구입니다.

- `feat(chat): 대화 기록 검색 추가`, `fix(ui): 다크 모드 버튼 색 수정`
- "추가", "수정", "제거", "변경"처럼 명사로 끝내고, 마침표를 붙이지 않습니다.
- 백틱을 쓰지 않습니다. `git log`와 터미널은 마크다운을 렌더링하지 않습니다.
- 용어와 문장은 `.agents/skills/korean-tech-writing/SKILL.md`를 따릅니다. 커밋에 필요한 건 용어집(`references/terms/common.md`)과 문장 규칙(`references/sentences.md`)이고, humanizer 단계는 커밋이라 생략합니다. Skill 도구가 있는 에이전트라면 `korean-tech-writing` 스킬을 불러도 됩니다. 두 스킬이 어긋나면 형식(type, scope, 콜론, 아래 형식 규칙)은 이 스킬이, 한국어 표현은 korean-tech-writing이 정합니다.

**본문**은 기본적으로 쓰지 않습니다. squash되면 개별 커밋 본문은 거의 보지 않습니다. 다만 diff만 보고는 **왜** 이렇게 했는지 알 수 없을 때만 씁니다. 우회 처리의 이유, 특정 값을 고른 근거, 일부러 하지 않은 것 같은 내용입니다. 무엇을 바꿨는지는 diff에 있으니 다시 적지 않습니다. 본문도 명사구 불릿이나 짧은 문장으로 씁니다.

**호환성이 깨지는 변경**이면 `feat!:`처럼 `!`를 붙이고 `BREAKING CHANGE:` footer에 무엇이 깨지는지 적습니다. 실제 버전 영향은 PR 제목에서 다시 판단하니, 커밋에서는 표시만 정확히 해 둡니다.

> [!IMPORTANT]
> **형식 규칙.** 커밋할 때 commitlint를 돌리지 않으므로, 아래 규칙은 메시지를 쓰면서 직접 지켜야 합니다. PR 제목 검사(`pr-check.yml`)가 쓰는 `commitlint.config.mjs`(`@commitlint/config-conventional` 기반)와 같은 규칙입니다. 커밋 메시지가 이 규칙을 지켜야 나중에 PR 제목으로 그대로 옮겨도 CI에서 막히지 않습니다. 커밋하기 전에 한 줄씩 대조합니다.
>
> 1. **설명의 첫 글자를 대문자로 쓰지 않습니다.** (`subject-case`) 실제로 가장 자주 걸리는 규칙입니다. README, API, Electron처럼 대문자로 시작하는 영어 단어가 설명 맨 앞에 오면 실패합니다. 어순을 바꿔 한국어로 시작하게 합니다.
>    - ✗ `docs: README에 커밋 규칙 안내 추가` → ✓ `docs: 커밋 규칙 안내를 README에 추가`
>    - ✗ `feat(ui): Electron 버전 복사 버튼 추가` → ✓ `feat(ui): 버전 복사 버튼 추가`
>    - ✗ `fix: API 키 노출 수정` → ✓ `fix: 노출된 API 키 제거`
>    - 소문자로 시작하는 이름은 괜찮습니다: `fix: useChatStore 리렌더링 수정`, `fix: macOS 알림 수정`
> 2. **type은 위 표의 11개 중 하나이고, 소문자입니다.** (`type-enum`, `type-case`) `Feat`, `FIX`는 실패합니다. `release`, `promote`는 쓰지 않습니다.
> 3. **형식은 `type(scope): 설명`입니다.** 콜론 뒤에 공백 한 칸을 넣고, scope는 영어 소문자로 씁니다. 설명은 비워 두지 않습니다(`subject-empty`).
> 4. **설명 끝에 마침표를 찍지 않습니다.** (`subject-full-stop`)
> 5. **header(첫 줄 전체)는 100자 이하이고, 앞뒤에 공백이 없습니다.** (`header-max-length`, `header-trim`) 100자를 넘기면 한 커밋에 너무 많은 걸 담았다는 신호이니, 커밋을 나눌 수 있는지 먼저 봅니다.
> 6. **본문과 footer는 header 다음에 빈 줄을 하나 두고 시작하고, 각 줄은 100자 이하입니다.** (`body-leading-blank`, `body-max-line-length`, `footer-max-line-length`)

### 4. 커밋

메시지는 파일 입력으로 넘깁니다. 한국어, 따옴표, 여러 줄이 섞여도 셸 이스케이프 문제가 없습니다.

```bash
git commit -F - <<'EOF'
feat(chat): 대화 기록 검색 추가
EOF
```

`.husky/pre-commit`이 `lint-staged`를 돌려 ESLint와 Prettier로 staging된 파일을 고칩니다.

- hook이 고친 내용은 lint-staged가 알아서 커밋에 포함합니다.
- hook이 실패하면 커밋은 만들어지지 않은 상태입니다. 원인(주로 ESLint 오류)을 고치고 다시 `git add` 한 뒤 **새로** 커밋합니다. `--amend`로 직전 커밋을 건드리지 않습니다.

### 5. 결과 보고

커밋마다 해시와 메시지를 한 줄씩 보여 줍니다. 남겨 둔 변경이 있으면 무엇을 왜 남겼는지 함께 적습니다. push는 하지 않습니다. 사용자가 요청할 때만 합니다.

### 6. AGENTS.md 맞추기 (백그라운드)

커밋으로 명령어, 구조, 브랜치 규칙, 스킬 목록이 바뀌면 `AGENTS.md`가 금방 낡습니다. `AGENTS.md`는 모든 에이전트가 읽는 문서라서 낡은 내용이 남으면 다음 작업이 틀린 전제로 시작합니다. 그래서 커밋을 마친 뒤 서브 에이전트를 백그라운드로 띄워 `AGENTS.md`를 맞추고, 고친 게 있으면 커밋까지 하게 합니다. 사용자는 기다리지 않습니다.

**건너뛰는 경우.** 아래에 해당하면 아무것도 하지 않고, 결과 보고에도 언급하지 않습니다.

- 에이전트에 서브 에이전트를 백그라운드로 띄우는 기능이 없을 때. 메인 흐름에서 대신 하지 않습니다. 커밋 요청이 문서 작업으로 길어지면 안 되기 때문입니다.
- 이번 커밋이 `AGENTS.md`나 `CLAUDE.md`만 바꿨을 때. 서브 에이전트가 만든 문서 커밋이 다시 이 단계를 부르는 것을 막기 위해서입니다.
- 커밋을 마친 뒤에도 `AGENTS.md`에 커밋되지 않은 변경이 남아 있을 때. 사용자가 고치던 내용과 섞이지 않게 합니다.

**서브 에이전트에 넘기는 것.** 서브 에이전트는 이 대화를 모르므로, 혼자 읽어도 할 일을 알 수 있게 씁니다.

- 이번에 만든 커밋 범위: 첫 커밋의 부모부터 마지막 커밋까지 (예: `abc123^..def456`). 첫 커밋이 저장소의 최초 커밋이면 부모가 없으니 커밋 해시 목록을 넘깁니다
- 할 일: 그 커밋들의 diff를 보고, `AGENTS.md`에서 실제 코드나 설정과 어긋난 부분, 새로 생겼는데 빠진 부분만 고친다. 문체와 용어는 `.agents/skills/korean-tech-writing/SKILL.md`를 따른다. 문서 전체를 다시 쓰지 않는다
- 고칠 게 없으면 아무것도 바꾸지 않고 "변경 없음"으로 보고한다
- 고쳤으면 `AGENTS.md`만 커밋한다. 메시지 형식은 `.agents/skills/commit/SKILL.md` 3단계의 IMPORTANT 블록을 따르고, type은 `docs`다. 특히 설명을 대문자로 시작하는 단어로 시작하면 안 되므로, "AGENTS.md에 ..."처럼 파일 이름으로 시작하지 않는다
  - ✗ `docs: AGENTS.md에 테스트 명령어 추가`
  - ✓ `docs: 에이전트 안내에 테스트 명령어 추가`
  ```bash
  git commit -F - -- AGENTS.md <<'EOF'
  docs: ...
  EOF
  ```
  경로를 지정해 커밋하면 사용자가 그사이 staging해 둔 다른 파일은 커밋에 섞이지 않고 그대로 남는다
- 이 단계(6단계)는 다시 실행하지 않는다. push하지 않는다
- 끝나면 커밋 해시와 무엇을 왜 고쳤는지 짧게 보고한다

서브 에이전트를 띄운 뒤에는 기다리지 않고 결과 보고를 마칩니다. 보고 끝에 "`AGENTS.md`를 백그라운드에서 점검하고 있다"고 한 줄 덧붙입니다. 서브 에이전트가 끝나면 문서 커밋이 생겼는지, 무엇을 고쳤는지 사용자에게 알립니다.

Claude Code라면 문서 커밋이 생긴 뒤 `/doctor prompt-audit AGENTS.md`를 실행해 보라고 함께 안내합니다. 이 명령은 지시 파일 안의 모순, 없는 파일이나 명령어를 가리키는 내용을 찾아 줍니다. 슬래시 명령어라 서브 에이전트가 대신 실행할 수 없습니다.

## 하지 않는 것

- `--no-verify`로 hook 건너뛰기, `--amend`, `reset --hard`, force push는 사용자가 명시적으로 요청할 때만 합니다.
- `git config`를 바꾸지 않습니다.
- 비밀 정보(`.env*`, 키, 토큰, 인증서)는 요청이 있어도 커밋하지 않고 알립니다.
- `release`, `promote` 타입과 `Release-As:` footer를 쓰지 않습니다.
- PR 제목을 정하거나 PR을 만들지 않습니다. 그건 별도 스킬의 일입니다.
- 6단계 서브 에이전트는 `AGENTS.md` 한 파일만 커밋합니다. 다른 파일을 함께 커밋하거나 push하지 않습니다.
- 커밋할 때 `pnpm exec commitlint`를 돌리지 않고, `commit-msg` hook도 추가하지 않습니다. 형식은 3단계 규칙으로 직접 지킵니다. 사람이 직접 하는 커밋은 검사하지 않는 게 이 저장소의 의도입니다.
