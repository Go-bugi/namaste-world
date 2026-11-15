# Spec Delta

## Purpose

앱을 켜면 시작 설정 창에서 설치된 에이전트 감지, 훅 설치 결과 확인, 사용할 에이전트와 모델 선택을 차례로 진행합니다. 고른 에이전트와 모델은 이후 질문을 보내는 기능이 쓸 수 있도록 앱이 보관합니다.

## ADDED Requirements

### Requirement: FR-START-001 앱 시작 때 시작 설정 창

앱은 실행되면 시작 설정 창(label `onboarding`)을 열어야 한다(SHALL). 앱을 켤 때 이 창 말고 다른 앱 창은 열지 않는다.

#### Scenario: 앱 실행

- **WHEN** 사용자가 앱을 실행함
- **THEN** 시작 설정 창 하나만 열리고, 그 창에는 Tauri 템플릿 문구와 링크, 버전 표시, 데모 3D, 훅 이벤트 목록이 없다

### Requirement: FR-START-002 단계 순서

시작 설정 창은 감지, 훅 설치, 에이전트와 모델 선택의 세 단계를 이 순서로 보여 주어야 한다(SHALL). 각 단계에는 지금 진행 중인지, 끝났는지가 보인다. 감지 단계는 FR-AGENT-007의 앱 시작 감지 결과를, 훅 설치 단계는 FR-HOOK-007의 설치 결과를 보여 준다.

#### Scenario: 감지와 설치가 진행 중

- **WHEN** 앱 시작 감지가 아직 끝나지 않음
- **THEN** 감지 단계는 진행 중으로, 훅 설치 단계와 선택 단계는 아직 시작하지 않은 것으로 보인다

#### Scenario: 감지와 설치가 끝남

- **WHEN** 앱 시작 감지와 그 뒤의 훅 설치가 끝남
- **THEN** 감지 단계와 훅 설치 단계는 끝난 것으로 보이고, 선택 단계에서 에이전트와 모델을 고를 수 있다

### Requirement: FR-START-003 에이전트 선택지

선택 단계는 감지 결과 중 설치됨인 항목만 에이전트 선택지로 보여 주어야 한다(SHALL). 선택지 하나는 에이전트와 실행 환경의 쌍이고, 에이전트 이름과 실행 환경을 함께 보여 준다. 설치된 항목이 하나뿐이면 그 항목을 미리 고른 상태로 둔다.

#### Scenario: 여러 환경에 설치됨

- **WHEN** Codex가 Windows와 WSL `Ubuntu`에 설치되어 있고, Claude Code는 설치되어 있지 않음
- **THEN** 선택지에 Windows의 Codex와 WSL `Ubuntu`의 Codex 두 개가 나오고 Claude Code는 나오지 않는다

#### Scenario: 하나만 설치됨

- **WHEN** 설치된 항목이 macOS의 Claude Code 하나뿐임
- **THEN** macOS의 Claude Code가 이미 골라진 상태로 보인다

#### Scenario: 설치된 에이전트가 없음

- **WHEN** 감지 결과에 설치됨인 항목이 하나도 없음
- **THEN** 선택지 대신 설치된 에이전트가 없다는 안내와 다시 감지 버튼이 보이고, 다음 버튼은 비활성화된다

### Requirement: FR-START-004 모델 선택지

선택 단계는 고른 에이전트의 모델 목록(FR-START-007)을 표시 이름으로 보여 주어야 한다(SHALL). CLI가 준 모델 설명문은 보여 주지 않고, 별칭인 모델(`default`, `opus` 등)에만 지금 가리키는 실제 모델 ID를 함께 보여 준다. 고정 목록을 쓸 때는 실제 모델 ID를 알 수 없으므로 이름만 보여 준다. 에이전트를 바꾸면 모델 선택지도 그 에이전트의 목록으로 바뀌고, 고른 모델은 그 에이전트의 기본 모델로 돌아간다. 다시 감지한 뒤에는 고른 에이전트가 여전히 설치됨이면 그대로 두고, 고른 모델이 새 목록에도 있으면 그대로 둔다. 없어진 쪽만 FR-START-003과 이 요구사항대로 다시 고른다. 기본 모델은 Claude Code는 `default`이고, Codex는 그 실행 환경 `config.toml`의 `model` 값이며 값이 없으면 목록의 첫 모델이다. 목록을 가져오는 동안에는 불러오는 중임을 보여 준다.

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

### Requirement: FR-START-007 모델 목록 조회

앱은 에이전트의 모델 목록을 그 실행 환경(WSL 배포판 포함)에 설치된 CLI에서 조회해야 한다(SHALL). Claude Code는 프롬프트 없이 초기화 요청만 보내 모델 목록을 받고, Codex는 모델 목록 명령의 결과 중 목록에 보이도록 표시된 모델만 쓴다. 조회는 사용 토큰을 쓰지 않고, 훅과 MCP 서버를 실행하지 않으며, 세션 기록을 남기지 않는다. 조회가 실패하거나 SC-START-001 안에 끝나지 않으면 앱에 둔 그 에이전트의 고정 목록을 쓴다. 감지(다시 감지 포함)가 끝나면 설치된 항목 중 WSL이 아닌 것을 모두 동시에 조회하고, WSL 항목은 사용자가 그 항목을 고를 때 조회하고, 다시 감지가 끝났을 때 이미 골라진 WSL 항목이 있으면 그 항목도 함께 다시 조회한다.

#### Scenario: Claude Code 조회

- **WHEN** macOS에 Claude Code가 설치되어 있고 Claude Code를 고름
- **THEN** 선택지에 `default`, `opus`, `sonnet`, `fable`, `haiku`와 Claude Code가 알려 준 이전 모델들이 나오고, 이때 앱의 훅 이벤트는 생기지 않는다

#### Scenario: Codex 조회

- **WHEN** Codex의 모델 목록 명령이 목록 표시 모델 3개와 숨김 모델 2개를 돌려줌
- **THEN** 선택지에는 목록 표시 모델 3개만 나온다

#### Scenario: 조회 시점

- **WHEN** Windows의 Codex와 WSL `Ubuntu`의 Codex가 설치됨으로 감지됨
- **THEN** 감지가 끝나자마자 Windows의 Codex 목록 조회가 시작되고, WSL `Ubuntu`의 Codex는 사용자가 그 항목을 고를 때 조회가 시작된다

#### Scenario: 골라 둔 WSL 항목을 다시 감지

- **WHEN** WSL `Ubuntu`의 Codex를 골라 모델 목록을 받은 뒤 다시 감지 버튼을 누름
- **THEN** 다시 감지가 끝나면 WSL `Ubuntu`의 Codex 모델 목록도 다시 조회되고, 그동안 모델 선택지는 불러오는 중으로 보인다

#### Scenario: 조회 실패

- **WHEN** CLI가 오류로 끝나거나 응답하지 않음
- **THEN** 시간 상한 뒤 그 에이전트의 고정 목록이 선택지로 나오고, 다음으로 넘어갈 수 있다

### Requirement: FR-START-005 다음 버튼

시작 설정 창은 에이전트와 모델이 모두 골라졌을 때만 다음 버튼을 활성화해야 한다(SHALL). 감지나 다시 감지가 진행 중이면 다음 버튼은 비활성화된다. 고른 에이전트의 훅 설치가 실패했어도 다음으로 넘어갈 수 있고, 이때 선택 단계에 그 실패가 함께 보인다.

#### Scenario: 둘 다 골라짐

- **WHEN** 에이전트와 모델이 모두 골라졌고 감지가 진행 중이 아님
- **THEN** 다음 버튼이 활성화된다

#### Scenario: 모델이 없음

- **WHEN** 에이전트는 골랐지만 모델이 골라지지 않음
- **THEN** 다음 버튼은 비활성화된다

#### Scenario: 고른 에이전트의 훅 설치 실패

- **WHEN** 고른 Codex의 훅 설치가 실패함
- **THEN** 선택 단계에 Codex의 설치 실패와 이유가 보이고, 다음 버튼은 활성화된다

### Requirement: FR-START-006 선택 보관

앱은 다음 버튼을 눌렀을 때의 에이전트, 실행 환경, 모델을 보관하고, 이후 열리는 창이 그 값을 읽을 수 있게 해야 한다(SHALL). 이 값은 앱 데이터 폴더에 저장한다. 다음 실행 때 시작 설정 창은 저장한 에이전트와 실행 환경이 선택지에 있으면 그것을, 저장한 모델이 모델 선택지에 있으면 그것을 미리 골라 둔다. 없으면 FR-START-003, FR-START-004대로 고른다.

#### Scenario: 작은 창이 선택을 읽음

- **WHEN** macOS의 Claude Code와 모델 하나를 고르고 다음을 누름
- **THEN** 새로 열린 창이 앱에서 읽은 선택 값은 macOS의 Claude Code와 그 모델이다

#### Scenario: 다음 실행 때 미리 골라 둠

- **WHEN** 지난 실행에서 macOS의 Codex와 `gpt-5.6-sol`을 골랐고, 이번 실행에서도 둘 다 선택지에 있음
- **THEN** 시작 설정 창이 열리고 선택 단계에 Codex와 `gpt-5.6-sol`이 골라진 상태로 보인다

#### Scenario: 저장한 에이전트가 사라짐

- **WHEN** 지난 실행에서 고른 Codex가 이번 감지에서 설치되지 않음으로 나옴
- **THEN** Codex는 골라지지 않고 FR-START-003대로 고른다

### Requirement: SC-START-001 모델 목록 조회 시간 상한

모델 목록 조회 한 번은 시작한 지 5초 안에 목록이나 고정 목록으로 끝나야 한다(MUST).

#### Scenario: CLI가 멈춤

- **WHEN** 모델 목록을 조회하는 CLI가 응답하지 않음
- **THEN** 조회를 시작한 지 5초 안에 고정 목록이 선택지에 나온다
