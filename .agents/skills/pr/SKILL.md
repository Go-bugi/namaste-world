---
name: pr
description: 이 저장소에서 지금 브랜치의 Pull Request를 만듭니다. 브랜치가 갈라져 나온 곳으로 base(develop, main, release/*)를 정하고, release-please 릴리스 노트에 그대로 들어갈 PR 제목을 쓰고, .github/PULL_REQUEST_TEMPLATE.md를 채운 본문을 보여 준 뒤 확인을 받고 push와 gh pr create까지 합니다. 머지되면 버전과 릴리스 노트가 어떻게 바뀌는지도 함께 알려 줍니다. 사용자가 "PR 올려 줘", "PR 만들어 줘", "풀리퀘 열어 줘", "PR 제목 정해 줘", "PR 본문 써 줘", "이 브랜치 develop에 올려", "리뷰 요청할 수 있게 해 줘", "/pr"이라고 하거나, 작업을 마치고 머지 준비를 해 달라고 할 때 반드시 이 스킬을 사용하세요. 이미 열린 PR의 제목이나 본문을 고칠 때도 씁니다. 커밋은 commit 스킬, 머지와 릴리스 단계 전환(Promote)은 이 스킬의 범위가 아닙니다.
---

# pr

## 이 스킬이 존재하는 이유

이 저장소는 release-please로 버전과 릴리스 노트를 만들고, 작업 브랜치는 squash merge합니다. 저장소 설정상 squash 커밋은 **제목이 PR 제목, 본문이 빈칸**입니다. 그래서 release-please가 읽는 건 PR 제목 한 줄뿐이고, 그 한 줄이 그대로 릴리스 노트 항목이 되고 버전을 얼마나 올릴지도 정합니다.

즉 PR 제목은 리뷰어가 아니라 **릴리스 노트를 읽는 사람**을 위한 글입니다. 본문은 릴리스 노트에 들어가지 않고 리뷰어가 읽습니다. 이 스킬은 두 독자를 구분해서 쓰고, 공개되는 작업(push, PR 생성)은 확인을 받은 뒤에 합니다.

## 브랜치 흐름

| 브랜치 | 갈라진 곳 | PR base | 머지 방식 |
|---|---|---|---|
| `feature/*` | `develop` | `develop` | squash |
| `hotfix/*` | `main` | `main` | squash |
| `fix/*` | `release/*` | 그 `release/*` | squash |
| `release/*` | `develop` (Promote 워크플로가 만듦) | `main` | **merge commit**, 정식 버전 Release PR을 머지한 뒤 |

봇이 여는 PR은 이 스킬이 다루지 않습니다. release-please Release PR(`release-please--*` 브랜치)과 `back-merge/*` PR은 워크플로가 자동으로 만듭니다. 사용자가 그런 브랜치에서 PR을 요청하면 자동으로 생성된다고 알려 주고 멈춥니다.

## 진행 순서

### 1. 상태 확인

```bash
git branch --show-current
git status --porcelain
git fetch origin
gh pr list --head "$(git branch --show-current)" --state open --json number,url,title,baseRefName
```

- **`main`이나 `develop`에 있으면** PR을 만들 수 없습니다. 작업 브랜치로 옮겨서 다시 요청하라고 안내합니다.
- **커밋되지 않은 변경이 있으면** 먼저 커밋할지 묻습니다. 커밋은 `.agents/skills/commit/SKILL.md`를 따릅니다. 커밋하지 않고 진행하겠다고 하면 그 변경은 PR에 들어가지 않는다고 알립니다.
- **이 브랜치에 이미 열린 PR이 있으면** 새로 만들지 않습니다. 기존 PR의 제목과 본문을 새로 쓴 내용으로 고칠지 묻습니다. 고칠 때는 5단계에서 `gh pr edit`을 씁니다.

### 2. base 정하기

`release/*`는 표대로 `main`입니다. 나머지는 **브랜치가 실제로 갈라져 나온 곳**으로 판단합니다. 브랜치 이름 규칙이 생긴 지 얼마 안 돼서 접두사만 믿으면 틀릴 수 있습니다.

후보(`origin/develop`, `origin/main`, `origin/release/*`)마다 양쪽에만 있는 커밋 수를 셉니다. 합이 가장 작은 후보가 갈라져 나온 곳입니다.

```bash
for b in develop main $(git branch -r --list 'origin/release/*' | sed 's|origin/||'); do
  echo "$b $(git rev-list --count --left-right "origin/$b...HEAD" | awk '{print $1+$2}')"
done
```

- 결과가 접두사와 맞으면 그대로 씁니다. 예를 들어 `feature/x`가 `develop`에서 갈라진 경우입니다.
- 결과와 접두사가 어긋나거나, 접두사가 표에 없으면 4단계 확인 때 base를 함께 묻습니다. 예를 들어 `feature/x`가 `main`에서 갈라졌다면 hotfix로 보내야 하는 변경일 수 있습니다.
- base보다 뒤처져 있으면(`origin/<base>`에만 있는 커밋이 있으면) 알리기만 합니다. rebase나 merge를 대신 하지 않습니다.

**base가 `main`이면 두 가지를 더 확인합니다.** main으로 가는 PR은 워크플로가 브랜치 이름을 보고 후속 작업을 하기 때문에, 이름과 상태가 맞지 않으면 PR이 머지돼도 흐름이 끊깁니다.

- **작업 브랜치는 이름이 `hotfix/`로 시작해야 합니다.** `hotfix-back-merge.yml`은 head 브랜치가 `hotfix/`로 시작할 때만 그 수정을 `develop`과 진행 중인 `release/*`에 back-merge합니다. 다른 이름으로 main에 머지하면 수정이 main에만 남고, 다음 정식 출시 때 같은 버그가 다시 나갑니다. 이름이 다르면 push하기 전에 바꾸자고 제안합니다.
  ```bash
  git branch -m hotfix/<이름>
  ```
- **`release/x.y.0`은 정식 버전이 나온 뒤에만 main으로 보냅니다.** 순서는 Promote 워크플로에서 stable을 실행하고, release 브랜치의 `x.y.0` Release PR을 머지하고, 그다음 release → main PR입니다. release 브랜치의 manifest 버전이 정식 버전인지 확인합니다.
  ```bash
  git show origin/release/x.y.0:.release-please-manifest.json   # "." 값이 x.y.0 이어야 함
  ```
  값이 `x.y.0-beta.2`, `x.y.0-rc.1`처럼 아직 pre-release이거나, `release-please-config.release.json`에 `release-as`가 남아 있으면 PR을 만들지 않습니다. 사용자에게 지금 단계를 알려 주고, Promote 워크플로에서 stable을 먼저 실행하라고 안내합니다. 이대로 머지하면 main에 정식 태그와 GitHub Release 없이 코드만 들어갑니다.

### 3. 제목과 본문 쓰기

먼저 브랜치 전체를 봅니다. 커밋 메시지만 보면 중간에 고친 내용이 빠지거나 겹칩니다.

```bash
git log --format='%h %s' origin/<base>..HEAD
git diff --stat origin/<base>...HEAD
git diff origin/<base>...HEAD
```

#### 제목

제목은 릴리스 노트에 이렇게 들어갑니다. scope는 굵게 표시되고, PR 번호가 붙습니다.

```markdown
### Features
* **chat:** 대화 기록 검색 추가 (#12)
```

이 한 줄만 보고 사용자가 무엇이 바뀌었는지 알 수 있어야 합니다.

- **사용자에게 바뀌는 점을 씁니다. 구현 방법은 쓰지 않습니다.** 구현 이야기는 본문에 씁니다.
  - ✗ `feat(renderer): useSearch 훅과 SearchBar 컴포넌트 추가` → ✓ `feat(chat): 대화 기록 검색 추가`
  - ✗ `fix: base.css hover 변수를 gray-1로 변경` → ✓ `fix(ui): 다크 모드에서 버튼 hover 색이 거의 안 바뀌던 문제 수정`
- **type은 사용자에게 미치는 영향이 가장 큰 것으로 고릅니다.** 브랜치 커밋 중에 `feat`가 하나라도 있으면 제목도 `feat`입니다. 제목에서 `feat`가 빠지면 그 기능은 릴리스 노트에 실리지 않습니다. 반대로 기능을 만들다 생긴 `fix` 커밋은 사용자가 겪은 버그가 아니니 제목에 반영하지 않습니다.
- **scope는 사용자가 알아보는 영역일 때만 붙입니다.** `chat`, `settings`, `ui`처럼 기능이나 화면 이름은 괜찮습니다. `renderer`, `preload`, `main`처럼 내부 구조 이름은 릴리스 노트 독자에게 의미가 없으니 생략합니다.
- **호환성이 깨지면 `feat!:`처럼 `!`를 붙입니다.** 본문은 커밋에 들어가지 않아서 `BREAKING CHANGE:` footer를 본문에 써도 release-please에 전달되지 않습니다.
- **형식 규칙은 commit 스킬과 같습니다.** `.agents/skills/commit/SKILL.md` 3단계의 IMPORTANT 블록을 읽고 그대로 지킵니다. PR 제목은 CI(`pr-check.yml`)가 commitlint로 검사하므로, 커밋보다 더 확실하게 지켜야 합니다. 특히 설명을 대문자로 시작하는 영어 단어(README, API, Electron)로 시작하지 않습니다.
- `release`, `promote` 타입은 Promote 워크플로 전용이라 CI가 막습니다. 쓰지 않습니다.

**`release/* → main` PR은 예외입니다.** merge commit으로 머지하므로 Conventional Commits 형식을 쓰면 main 릴리스 노트에 한 번 더 기록됩니다. 그래서 `Release 1.2.0`처럼 씁니다. 버전은 브랜치 이름에서 가져옵니다. CI도 이 PR은 제목 검사를 건너뜁니다.

**기능이 둘 이상 섞인 PR**이면 먼저 PR을 나눌지 제안합니다. 사용자가 나누지 않겠다고 하면, 본문 맨 아래에 다음 블록을 넣어 릴리스 노트 항목을 나눕니다. release-please가 squash 커밋 대신 이 내용을 읽습니다. 블록 안의 각 줄도 제목과 같은 규칙으로 씁니다.

```
BEGIN_COMMIT_OVERRIDE
feat(chat): 대화 기록 검색 추가
feat(settings): 테마 설정 추가
END_COMMIT_OVERRIDE
```

#### 본문

`.github/PULL_REQUEST_TEMPLATE.md`를 읽고 그 구조로 채웁니다. 템플릿이 바뀌면 바뀐 구조를 따릅니다.

- 템플릿의 `<!-- -->` 안내 주석은 지웁니다. 사람이 직접 쓸 때 보는 안내라 채운 본문에는 필요 없습니다.
- **해당 없는 섹션은 지웁니다.** 예를 들어 화면 변경이 없으면 "스크린샷", 이슈가 없으면 "관련 이슈"를 지웁니다.
- **확인하지 않은 내용은 쓰지 않습니다.** 체크리스트는 이 스킬이 실제로 확인한 항목만 체크합니다. `pnpm lint`나 `pnpm typecheck`는 돌리지 않으므로(CI가 돌립니다) 그 항목은 체크하지 않은 채로 둡니다. "확인 방법"도 diff에서 확인되는 절차만 씁니다.
- 스크린샷은 직접 찍을 수 없으니, 화면이 바뀌었으면 섹션을 남겨 두고 사용자에게 첨부를 부탁한다고 4단계에서 알립니다.
- 문장은 `.agents/skills/korean-tech-writing/SKILL.md`를 따릅니다. PR 본문은 습니다체로 쓰고, 불릿 항목은 명사구로 끝냅니다. 코드 이름에는 백틱을 씁니다(GitHub가 렌더링합니다). 개요가 여러 문단이면 그 스킬의 humanizer 단계도 거칩니다.
- `release/* → main` PR은 본문 맨 위에 "**merge commit으로 머지해 주세요. squash하면 안 됩니다.**"를 넣습니다. 이 저장소의 브랜치 흐름에서 release → main은 merge commit으로 머지하기로 정해져 있는데, 다른 PR은 모두 squash라서 버튼을 잘못 누르기 쉽습니다.

### 4. 확인받기

PR은 공개되고, 제목은 릴리스 노트에 남습니다. 만들기 전에 아래 내용을 보여 주고 답을 받습니다.

```
base: develop ← feature/chat-search (squash merge)
제목: feat(chat): 대화 기록 검색 추가

릴리스 영향: feat라서 develop에 머지되면 Release PR이 생기고 다음 alpha 버전에
"Features: chat: 대화 기록 검색 추가"로 실립니다.

본문:
(채운 본문 전체)
```

**릴리스 영향**은 base와 type으로 정합니다.

| base | type | 머지되면 |
|---|---|---|
| `develop` | `feat`, `fix`, `perf`, `revert` | develop Release PR이 생기거나 갱신되고, 다음 alpha에 실림 |
| `release/*` | `fix`, `perf`, `revert` | release Release PR이 갱신되고, 다음 beta나 rc에 실림 |
| `release/*` | `feat` | 안정화 단계에 기능이 들어감. 의도한 것인지 확인 |
| `main` | `fix`, `perf`, `revert` | patch 버전(예: 0.1.1) Release PR이 생김. 머지 후 develop과 진행 중인 release로 자동 back-merge |
| `main` | `feat` | hotfix로 minor 버전이 오름. 의도한 것인지 확인 |
| 어디든 | `refactor`, `style`, `docs`, `test`, `build`, `ci`, `chore` | 릴리스 노트에 나오지 않음. 이것만 머지되면 Release PR도 생기지 않음 |
| `main` | `Release x.y.0` (release 브랜치) | 정식 버전 Release PR이 생김. 머지 후 develop으로 back-merge PR이 자동 생성 |

### 5. 만들기

사용자가 승인하면 push하고 PR을 만듭니다. 본문은 heredoc으로 넘깁니다. 한국어, 백틱, 여러 줄이 섞여도 셸 이스케이프 문제가 없습니다.

```bash
git push -u origin HEAD
gh pr create --base <base> --title '<제목>' --body-file - <<'EOF'
(본문)
EOF
```

기존 PR을 고칠 때는 `gh pr edit <번호> --title '<제목>' --body-file -`를 씁니다.

만든 뒤 PR URL을 알려 줍니다. 화면 변경이 있었으면 스크린샷을 첨부해 달라고 함께 말합니다.

## 하지 않는 것

- 확인 없이 push하거나 PR을 만들지 않습니다.
- 머지하지 않고, auto-merge도 켜지 않습니다.
- force push, rebase, base 브랜치 merge는 사용자가 요청할 때만 합니다.
- 리뷰어, 라벨, assignee, draft 설정은 사용자가 요청할 때만 합니다.
- Release PR, back-merge PR 같은 봇 PR을 만들거나 고치지 않습니다.
- `release`, `promote` 타입과 `Release-As:`를 쓰지 않습니다.
- PR 본문에 `BREAKING CHANGE:`나 추가 `feat:` 줄을 써서 릴리스 노트를 바꾸려 하지 않습니다. 효과가 없습니다. 필요하면 제목의 `!`나 `BEGIN_COMMIT_OVERRIDE`를 씁니다.
