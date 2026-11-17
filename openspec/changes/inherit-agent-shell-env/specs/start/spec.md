## MODIFIED Requirements

### Requirement: FR-START-002 단계 순서

시작 설정 창은 감지, 훅 설치, 에이전트와 모델 선택, 로그인 확인의 네 단계를 이 순서의 스테퍼로 보여 주어야 한다(SHALL). Windows에서는 맨 앞에 셸 선택 단계(FR-START-011)를 더해 다섯 단계를 보여 준다. 한 번에 한 단계의 내용만 보이고, 창이 열리면 첫 단계(macOS는 감지, Windows는 셸 선택)에서 시작한다. 단계 표시줄에는 모든 단계가 나오고, 각 단계가 지금 보고 있는 단계인지, 끝났는지, 아직 시작하지 않았는지가 보인다. 단계는 이전과 다음 버튼(FR-START-005)으로만 옮긴다. 감지 단계는 FR-AGENT-007의 앱 시작 감지 결과를, 훅 설치 단계는 FR-HOOK-007의 설치 결과를 보여 준다.

#### Scenario: 창이 열림

- **WHEN** macOS에서 시작 설정 창이 열림
- **THEN** 감지 단계의 내용만 보이고, 단계 표시줄에 네 단계가 순서대로 나오며 감지 단계가 지금 단계로 표시된다

#### Scenario: 감지와 설치가 진행 중

- **WHEN** macOS에서 앱 시작 감지가 아직 끝나지 않음
- **THEN** 단계 표시줄에서 감지 단계는 지금 단계로, 훅 설치, 선택, 로그인 확인 단계는 아직 시작하지 않은 것으로 보인다

#### Scenario: 감지와 설치가 끝남

- **WHEN** 앱 시작 감지와 그 뒤의 훅 설치가 끝났고 사용자가 선택 단계까지 다음 버튼으로 넘어옴
- **THEN** 감지 단계와 훅 설치 단계는 끝난 것으로 보이고, 선택 단계에서 에이전트와 모델을 고를 수 있다

#### Scenario: 감지가 끝나도 자동으로 넘어가지 않음

- **WHEN** 감지 단계에 있는 동안 앱 시작 감지가 끝남
- **THEN** 감지 단계에 그대로 머물고 결과 목록이 보이며, 감지 단계는 끝난 것으로 표시된다

#### Scenario: 다음 단계로 이동

- **WHEN** 감지 단계에서 다음 버튼을 누름
- **THEN** 훅 설치 단계의 내용만 보이고, 단계 표시줄에서 감지 단계는 끝남, 훅 설치 단계는 지금 단계로 표시된다

#### Scenario: 이전 단계로 돌아감

- **WHEN** 로그인 확인 단계에서 이전 버튼을 누름
- **THEN** 에이전트와 모델 선택 단계가 보이고, 앞서 고른 에이전트와 모델이 그대로 골라져 있다

#### Scenario: Windows에서 창이 열림

- **WHEN** Windows에서 시작 설정 창이 열림
- **THEN** 셸 선택 단계의 내용만 보이고, 단계 표시줄에 셸 선택, 감지, 훅 설치, 선택, 로그인 확인의 다섯 단계가 순서대로 나오며 셸 선택 단계가 지금 단계로 표시된다

#### Scenario: Windows에서 셸 선택 다음

- **WHEN** Windows의 셸 선택 단계에서 다음 버튼을 누름
- **THEN** 감지 단계의 내용만 보이고, 셸 선택 단계는 끝남, 감지 단계는 지금 단계로 표시된다

### Requirement: FR-START-004 모델 선택지

선택 단계는 고른 에이전트의 모델 목록(FR-START-007)을 표시 이름으로 보여 주어야 한다(SHALL). CLI가 준 모델 설명문은 보여 주지 않고, 별칭인 모델(`default`, `opus` 등)에만 지금 가리키는 실제 모델 ID를 함께 보여 준다. 고정 목록을 쓸 때는 실제 모델 ID를 알 수 없으므로 이름만 보여 준다. 에이전트를 바꾸면 모델 선택지도 그 에이전트의 목록으로 바뀌고, 고른 모델은 그 에이전트의 기본 모델로 돌아간다. 다시 감지한 뒤에는 고른 에이전트가 여전히 설치됨이면 그대로 두고, 고른 모델이 새 목록에도 있으면 그대로 둔다. 없어진 쪽만 FR-START-003과 이 요구사항대로 다시 고른다. 기본 모델은 Claude Code는 `default`이고, Codex는 그 실행 환경 Codex 설정 폴더(FR-HOOK-002와 같은 폴더)의 `config.toml`에 있는 `model` 값이며 값이 없으면 목록의 첫 모델이다. 목록을 가져오는 동안에는 불러오는 중임을 보여 준다.

#### Scenario: 에이전트를 바꿈

- **WHEN** Claude Code를 고르고 `sonnet`을 고른 뒤 Codex로 바꿈
- **THEN** 모델 선택지가 Codex의 모델 목록으로 바뀌고 `config.toml`의 `model` 값이 골라진 상태가 된다

#### Scenario: 다시 감지 뒤 선택 유지

- **WHEN** macOS의 Claude Code와 `sonnet`을 고른 뒤 다시 감지했고, Claude Code는 여전히 설치됨이며 새 목록에도 `sonnet`이 있음
- **THEN** Claude Code와 `sonnet`이 계속 골라진 상태다

#### Scenario: Claude Code 기본 모델

- **WHEN** Claude Code를 고름
- **THEN** `default` 모델이 골라진 상태이고, 그 선택지에는 지금 가리키는 실제 모델 ID(예: `claude-opus-5-5`)가 함께 보이며 설명문은 보이지 않는다

#### Scenario: 고정 목록의 별칭

- **WHEN** Claude Code 모델 목록 조회가 실패해 고정 목록을 씀
- **THEN** `default`, `opus` 같은 별칭 선택지에는 이름만 보이고 실제 모델 ID는 보이지 않는다

#### Scenario: Codex 모델에는 설명문이 없음

- **WHEN** Codex를 고름
- **THEN** 모델 선택지에는 표시 이름(예: `GPT-5.6-Sol`)만 보이고 CLI가 준 설명문은 보이지 않는다

#### Scenario: Codex 설정의 모델이 목록에 없음

- **WHEN** Codex `config.toml`의 `model`이 `gpt-5.5`이고 조회한 목록에는 `gpt-5.5`가 없음
- **THEN** `gpt-5.5`가 선택지에 더해지고 골라진 상태가 된다

#### Scenario: 셸에서 바꾼 Codex 설정 폴더

- **WHEN** macOS 사용자가 `.zshrc`에 `export CODEX_HOME=~/codex-work`를 적었고, `~/codex-work/config.toml`의 `model`은 `gpt-5.6-sol`, `~/.codex/config.toml`의 `model`은 `gpt-5.5`이며, Codex를 고름
- **THEN** `gpt-5.6-sol`이 골라진 상태가 된다

### Requirement: FR-START-005 다음 버튼

시작 설정 창은 지금 단계를 넘어갈 조건이 채워졌을 때만 다음 버튼을, 돌아갈 수 있을 때만 이전 버튼을 활성화해야 한다(SHALL). 비활성화된 버튼은 눌리지 않는다. 다음 버튼의 조건은 셸 선택 단계는 항상, 감지 단계는 감지나 다시 감지가 진행 중이 아니고 설치됨인 항목이 하나 이상일 때, 훅 설치 단계는 설치가 끝났을 때, 선택 단계는 에이전트와 모델이 모두 골라졌을 때, 로그인 확인 단계는 결과가 로그인됨일 때다. 로그인 확인 단계의 다음 버튼은 FR-START-006의 보관과 작은 창 전환을 하고, 그 처리가 진행 중이면 두 버튼 모두 비활성화된다. 훅 설치(다시 설치 포함)가 진행 중이어도 두 버튼 모두 비활성화된다. 이전 버튼은 첫 단계(macOS는 감지, Windows는 셸 선택)에서 비활성화된다. 고른 에이전트의 훅 설치가 실패했어도 다음으로 넘어갈 수 있고, 이때 선택 단계에 그 실패가 함께 보인다.

#### Scenario: 감지 중

- **WHEN** 감지 단계에서 앱 시작 감지나 다시 감지가 진행 중임
- **THEN** 다음 버튼과 이전 버튼이 모두 비활성화된다

#### Scenario: 설치된 에이전트가 없음

- **WHEN** 감지가 끝났고 설치됨인 항목이 하나도 없음
- **THEN** 감지 단계의 다음 버튼은 비활성화된다

#### Scenario: 훅 설치 중

- **WHEN** 훅 설치 단계에 있고 설치가 아직 끝나지 않음
- **THEN** 다음 버튼과 이전 버튼이 모두 비활성화되고, 설치가 끝나면 둘 다 활성화된다

#### Scenario: 둘 다 골라짐

- **WHEN** 선택 단계에서 에이전트와 모델이 모두 골라짐
- **THEN** 다음 버튼이 활성화된다

#### Scenario: 모델이 없음

- **WHEN** 선택 단계에서 에이전트는 골랐지만 모델이 골라지지 않음
- **THEN** 다음 버튼은 비활성화된다

#### Scenario: 로그인 안 됨

- **WHEN** 로그인 확인 단계의 결과가 로그인 안 됨이거나 확인 실패이거나 아직 확인 중임
- **THEN** 다음 버튼은 비활성화되고 이전 버튼은 활성화된다

#### Scenario: 로그인됨

- **WHEN** 로그인 확인 단계의 결과가 로그인됨임
- **THEN** 다음 버튼이 활성화되고, 누르면 선택을 보관하고 작은 창으로 넘어간다

#### Scenario: 고른 에이전트의 훅 설치 실패

- **WHEN** 고른 Codex의 훅 설치가 실패함
- **THEN** 선택 단계에 Codex의 설치 실패와 이유가 보이고, 선택 단계의 다음 버튼은 에이전트와 모델이 골라졌으면 활성화된다

#### Scenario: Windows 셸 선택 단계

- **WHEN** Windows의 셸 선택 단계에 있음
- **THEN** 다음 버튼은 활성화되고 이전 버튼은 비활성화된다

#### Scenario: Windows 감지 단계의 이전 버튼

- **WHEN** Windows의 감지 단계에 있고 감지가 진행 중이 아님
- **THEN** 이전 버튼이 활성화되고, 누르면 셸 선택 단계로 돌아간다

## ADDED Requirements

### Requirement: FR-START-011 Windows 셸 선택 단계

Windows에서 시작 설정 창의 셸 선택 단계는 에이전트 CLI를 실행할 환경을 읽을 셸(FR-AGENT-014)을 고르게 해야 한다(SHALL). 선택지는 PowerShell, 명령 프롬프트, Git Bash이고, Git Bash는 Git for Windows가 설치되어 있을 때만 나온다. 한 번에 하나만 고를 수 있고, 각 선택지에는 그 셸의 설정 파일에 적은 환경 변수가 에이전트에 전달되는지 짧게 보여 준다. macOS에는 이 단계가 없다.

#### Scenario: 처음 실행

- **WHEN** Windows에서 앱을 처음 실행하고 Git for Windows가 설치되어 있음
- **THEN** 셸 선택 단계에 PowerShell, 명령 프롬프트, Git Bash가 나오고 PowerShell이 골라져 있다

#### Scenario: Git Bash가 없음

- **WHEN** Windows에 Git for Windows가 설치되어 있지 않음
- **THEN** 셸 선택 단계에 PowerShell과 명령 프롬프트만 나온다

#### Scenario: macOS

- **WHEN** macOS에서 시작 설정 창이 열림
- **THEN** 셸 선택 단계는 단계 표시줄과 내용 어디에도 나오지 않는다

### Requirement: FR-START-012 셸 선택 보관

앱은 셸 선택 단계에서 고른 셸을 고르는 즉시 앱 데이터 폴더에 저장하고, 다음 실행의 셸 선택 단계에 그 값을 미리 골라 두어야 한다(SHALL). 저장한 값이 없으면 PowerShell을 쓴다. 저장한 셸이 Git Bash인데 Git for Windows를 찾지 못하면 PowerShell을 쓰고, 셸 선택 단계에도 PowerShell이 골라진 상태로 보인다. 셸 선택 외의 셸 환경 값은 저장하지 않는다(FR-AGENT-017).

#### Scenario: 다음 실행 때 미리 골라 둠

- **WHEN** 지난 실행에서 Git Bash를 골랐고 Git for Windows가 여전히 설치되어 있음
- **THEN** 셸 선택 단계에 Git Bash가 골라져 있고, 그대로 다음을 누르면 Git Bash의 환경으로 감지한다

#### Scenario: 저장한 Git Bash가 사라짐

- **WHEN** 지난 실행에서 Git Bash를 골랐는데 그 뒤 Git for Windows를 삭제함
- **THEN** 셸 선택 단계에 PowerShell이 골라져 있다

### Requirement: FR-START-013 Windows 감지 시작 시점

Windows에서 앱은 셸 선택 단계의 다음 버튼이 눌릴 때 고른 셸의 환경으로 감지해야 한다(SHALL). 처음 누를 때는 앱 시작 감지(FR-AGENT-007)를 시작하고, 이전으로 돌아와 마지막 감지와 다른 셸을 고르고 다시 누르면 FR-AGENT-007의 다시 감지를 시작하되, 앱 시작 감지처럼 멈춘 WSL 배포판은 켜지 않는다(FR-AGENT-002). 마지막 감지와 같은 셸이면 다시 감지하지 않는다. 셸을 고르기만 해서는 감지하지 않는다. 감지가 끝나면 FR-HOOK-002대로 훅 설치를 확인하고, 이후 단계의 모델 목록과 로그인 확인은 그 셸의 환경을 쓴다.

#### Scenario: 처음 다음을 누름

- **WHEN** Windows의 셸 선택 단계에서 Git Bash를 고르고 다음 버튼을 누름
- **THEN** 감지 단계로 넘어가 확인 중이 보이고, Git Bash의 환경으로 한 감지가 끝나면 결과가 보인다

#### Scenario: 고르기만 함

- **WHEN** 셸 선택 단계에서 PowerShell에서 Git Bash로, 다시 명령 프롬프트로 바꾸기만 하고 다음을 누르지 않음
- **THEN** 감지는 시작되지 않는다

#### Scenario: 돌아와서 셸을 바꿈

- **WHEN** PowerShell로 감지를 마친 뒤 이전 버튼으로 셸 선택 단계로 돌아와 Git Bash를 고르고 다음 버튼을 누름
- **THEN** Git Bash의 환경으로 다시 감지가 시작되고, 감지 단계에는 확인 중 표시와 함께 이전 결과가 보이며, 다시 감지가 끝나면 새 결과로 바뀐다

#### Scenario: 셸을 바꿔도 멈춘 배포판은 켜지 않음

- **WHEN** WSL 배포판 `Ubuntu`가 Stopped 상태이고, PowerShell로 감지를 마친 뒤 셸 선택 단계로 돌아와 Git Bash를 고르고 다음 버튼을 누름
- **THEN** `Ubuntu`는 켜지지 않고, 다시 감지 결과에서 WSL `Ubuntu`의 Claude Code와 Codex는 멈춤으로 나온다

#### Scenario: 돌아왔지만 같은 셸

- **WHEN** PowerShell로 감지를 마친 뒤 셸 선택 단계로 돌아와 PowerShell 그대로 다음 버튼을 누름
- **THEN** 다시 감지는 시작되지 않고 감지 단계에 이전 결과가 그대로 보인다

#### Scenario: 바꾼 셸로 로그인 확인

- **WHEN** Git Bash의 `.bashrc`에만 `ANTHROPIC_API_KEY`가 있고, 셸 선택 단계에서 Git Bash를 골라 진행한 뒤 Windows의 Claude Code를 골라 로그인 확인 단계에 들어감
- **THEN** 로그인 확인 결과는 로그인됨이다
