## MODIFIED Requirements

### Requirement: FR-AGENT-007 앱 시작 때 감지하고 결과를 보관

앱은 시작할 때 감지를 한 번 하고 그 결과를 앱이 켜져 있는 동안 보관해야 한다(SHALL). Windows에서는 앱을 켤 때 감지하지 않고, 시작 설정 창의 셸 선택 단계에서 처음 다음 버튼을 누를 때 고른 셸(FR-START-011)로 감지하며, 이 감지를 앱 시작 감지로 본다. renderer가 결과를 요청하면 보관한 결과를 준다. renderer가 다시 감지를 요청하면 새로 감지하고 보관한 결과를 바꾼다. 결과는 디스크에 저장하지 않는다.

#### Scenario: 보관한 결과 요청

- **WHEN** 앱 시작 때 감지가 끝났고 renderer가 결과를 요청함
- **THEN** 새로 감지하지 않고 보관한 결과를 준다

#### Scenario: 시작 감지가 아직 진행 중

- **WHEN** 앱 시작 때 감지가 끝나기 전에 renderer가 결과를 요청함
- **THEN** 그 감지가 끝나면 결과를 준다

#### Scenario: 다시 감지

- **WHEN** 앱 시작 뒤 `codex`를 새로 설치하고 renderer가 다시 감지를 요청함
- **THEN** 새로 감지한 결과에 Codex가 설치됨으로 나오고, 이후 결과 요청도 이 결과를 준다

#### Scenario: 다시 감지 중 결과 요청

- **WHEN** 이전 결과를 보관한 상태에서 다시 감지가 진행 중이고 renderer가 결과를 요청함
- **THEN** 다시 감지가 끝나기를 기다리지 않고 이전 결과를 준다

#### Scenario: 앱 재시작

- **WHEN** 앱을 종료했다가 다시 실행함
- **THEN** 이전 실행의 결과를 쓰지 않고 시작 감지를 새로 한다

#### Scenario: Windows는 셸 선택 뒤에 감지

- **WHEN** Windows에서 앱을 실행하고 시작 설정 창이 셸 선택 단계에 머물러 있음
- **THEN** 감지는 시작되지 않고, 셸 선택 단계에서 다음 버튼을 누르면 고른 셸의 환경으로 감지가 시작된다

### Requirement: FR-AGENT-011 자격 증명을 다루지 않는 로그인 확인

앱은 로그인 상태를 확인할 때 CLI가 저장한 로그인 파일, 운영체제 키체인, 토큰을 직접 읽지 않아야 한다(MUST). 로그인 명령을 실행하지 않고, 모델을 호출하지 않아 사용 토큰을 쓰지 않으며, 확인 결과를 디스크에 저장하지 않는다. 셸 설정 파일에서 읽은 환경 변수(FR-AGENT-012~014)는 에이전트 CLI 프로세스에 그대로 넘기는 데만 쓰고, 앱이 그 값을 해석하거나 CLI 실행이 아닌 요청과 처리에 쓰지 않는다.

#### Scenario: 확인 중 키체인 접근 없음

- **WHEN** macOS에서 Claude Code의 로그인 상태를 확인함
- **THEN** 앱이 키체인 접근 권한을 묻는 창이 뜨지 않고, 사용량이 늘지 않는다

#### Scenario: 로그인 안 됨이어도 로그인 명령은 실행하지 않음

- **WHEN** 확인 결과가 로그인 안 됨임
- **THEN** 앱은 로그인 명령을 실행하지 않고 브라우저도 열지 않는다

#### Scenario: 셸 환경의 API 키는 CLI에만 전달

- **WHEN** 셸 설정 파일에 `ANTHROPIC_API_KEY`가 있고 앱이 Claude Code의 로그인 상태를 확인함
- **THEN** 그 키는 앱이 실행한 Claude Code 프로세스의 환경에만 들어가고, 앱은 그 키로 네트워크 요청을 보내거나 키 값으로 로그인 여부를 판단하지 않는다

## ADDED Requirements

### Requirement: FR-AGENT-012 macOS 로그인 셸 환경으로 CLI 실행

macOS에서 앱은 에이전트 CLI를 실행할 때(감지, 모델 목록 조회, 로그인 확인, 이후 기능) 사용자 로그인 셸의 환경 변수 전체를 넘겨야 한다(SHALL). 앱이 Finder나 Dock으로 실행되어 셸 설정 파일의 변수를 물려받지 못해도, 셸 설정 파일에만 적은 변수가 CLI에 전달된다. 앱 자신의 환경에만 있고 로그인 셸에 없는 변수는 그대로 남는다.

#### Scenario: 셸 설정 파일의 설정 폴더 변경

- **WHEN** 사용자가 `.zshrc`에 `export CLAUDE_CONFIG_DIR=~/work-claude`를 적고 그 폴더로 Claude Code에 로그인했으며, 앱은 Finder에서 실행됨
- **THEN** macOS Claude Code의 로그인 확인 결과는 로그인됨이다

#### Scenario: 셸 설정 파일의 API 키

- **WHEN** 사용자가 `.zshrc`에만 `export OPENAI_API_KEY=...`를 적었고 Codex는 그 키로 인증하며, 앱은 Dock에서 실행됨
- **THEN** macOS Codex의 로그인 확인 결과는 로그인됨이다

#### Scenario: 이슈에 없던 변수도 전달

- **WHEN** 사용자가 `.zshrc`에 `export ANTHROPIC_BASE_URL=https://proxy.example`을 적음
- **THEN** 앱이 실행한 Claude Code도 같은 `ANTHROPIC_BASE_URL` 값을 받는다

### Requirement: FR-AGENT-013 WSL 로그인 셸 설정으로 CLI 실행

WSL 배포판에서 앱은 에이전트 CLI를 그 배포판 사용자의 로그인 셸로 실행해야 한다(SHALL). 로그인 셸이 bash나 zsh면 대화형 로그인 셸로 실행해 `.bashrc`, `.zshrc`도 읽는다. 로그인 셸이 그 밖의 셸이거나 알 수 없으면 `/bin/sh` 로그인 셸로 실행한다. 감지, 모델 목록 조회, 로그인 확인에 모두 같은 방식을 쓴다.

#### Scenario: .bashrc의 환경 변수

- **WHEN** WSL `Ubuntu` 사용자의 로그인 셸이 bash이고 `.bashrc`에 `export CODEX_HOME=~/codex-work`를 적었으며 그 폴더로 Codex에 로그인함
- **THEN** WSL `Ubuntu` Codex의 로그인 확인 결과는 로그인됨이다

#### Scenario: .zshrc로만 PATH에 들어간 에이전트

- **WHEN** WSL `Debian` 사용자의 로그인 셸이 zsh이고 `claude`가 `.zshrc`가 `PATH`에 더한 폴더에만 있음
- **THEN** WSL `Debian`의 Claude Code가 설치됨으로 나온다

#### Scenario: 로그인 셸이 sh

- **WHEN** WSL 배포판 사용자의 로그인 셸이 `/bin/sh`임
- **THEN** CLI는 `/bin/sh` 로그인 셸로 실행되고 `.profile`의 변수가 전달된다

### Requirement: FR-AGENT-014 Windows에서 고른 셸의 환경으로 CLI 실행

Windows 실행 환경에서 앱은 에이전트 CLI를 시작 설정 창에서 고른 셸(FR-START-011)이 설정 파일을 읽은 뒤의 환경 변수 전체로 실행해야 한다(SHALL). PowerShell이면 PowerShell 7이 설치되어 있을 때 그것의 프로필을, 없으면 Windows PowerShell의 프로필을 읽는다. Git Bash면 대화형 로그인 셸로 `.bash_profile`, `.bashrc`를 읽는다. 명령 프롬프트면 앱이 받은 환경을 그대로 쓴다. WSL 실행 환경은 이 선택과 상관없이 FR-AGENT-013을 따른다.

#### Scenario: PowerShell 프로필의 API 키

- **WHEN** 셸로 PowerShell을 골랐고, PowerShell 프로필에만 `$env:ANTHROPIC_API_KEY = "..."`를 적었으며 Claude Code는 그 키로 인증함
- **THEN** Windows Claude Code의 로그인 확인 결과는 로그인됨이다

#### Scenario: Git Bash .bashrc의 설정 폴더

- **WHEN** 셸로 Git Bash를 골랐고 `.bashrc`에 `export CODEX_HOME=/d/codex`를 적었으며 그 폴더로 Codex에 로그인함
- **THEN** Windows Codex의 로그인 확인 결과는 로그인됨이다

#### Scenario: 명령 프롬프트

- **WHEN** 셸로 명령 프롬프트를 골랐고 `ANTHROPIC_API_KEY`는 Windows 사용자 환경 변수에 있음
- **THEN** 앱은 셸을 실행하지 않고 받은 환경으로 Claude Code를 실행하며, 결과는 로그인됨이다

#### Scenario: WSL은 선택과 무관

- **WHEN** 셸로 Git Bash를 골랐고 WSL `Ubuntu`에도 Codex가 있음
- **THEN** WSL `Ubuntu`의 Codex는 FR-AGENT-013대로 배포판 사용자의 로그인 셸로 실행된다

### Requirement: FR-AGENT-015 셸 환경을 읽는 시점

앱은 셸 환경(FR-AGENT-012, FR-AGENT-014)을 앱 시작 감지(Windows는 셸 선택 뒤의 감지, FR-AGENT-007)와 다시 감지 때 읽고, 그 뒤의 모델 목록 조회와 로그인 확인은 마지막으로 읽은 환경을 써야 한다(SHALL). 모델 목록 조회와 로그인 확인은 셸을 새로 실행하지 않는다. 아직 읽은 환경이 없으면 앱이 받은 환경을 쓴다.

#### Scenario: 다시 감지로 바뀐 값 반영

- **WHEN** 앱이 켜진 뒤 사용자가 `.zshrc`에 `CLAUDE_CONFIG_DIR`을 추가하고 다시 감지를 누른 다음 로그인 상태를 다시 확인함
- **THEN** 로그인 확인은 새로 추가한 `CLAUDE_CONFIG_DIR`로 실행한 CLI의 결과를 보여 준다

#### Scenario: 로그인 확인은 셸을 띄우지 않음

- **WHEN** 감지가 끝난 뒤 사용자가 로그인 확인 단계에서 다시 확인을 누름
- **THEN** 셸은 실행되지 않고 감지 때 읽은 환경으로 CLI가 실행된다

### Requirement: FR-AGENT-016 셸 환경을 읽지 못하면 받은 환경으로 진행

앱은 셸 환경을 읽지 못했을 때(셸 실행 실패, 셸 설정 파일 오류로 비정상 종료, 알아볼 수 없는 출력, SC-AGENT-003 시간 초과) 앱이 받은 환경으로 CLI를 실행해야 한다(SHALL). 이 실패만으로 감지 결과를 확인 실패로 만들지 않는다.

#### Scenario: 셸 설정 파일이 오류로 끝남

- **WHEN** 사용자의 `.zshrc`가 오류로 끝나 셸이 0이 아닌 코드로 종료됨
- **THEN** 앱은 받은 환경으로 감지를 계속하고, 그 환경에서 찾은 에이전트는 설치됨으로 나온다

#### Scenario: PowerShell 프로필이 오류로 끝남

- **WHEN** 셸로 PowerShell을 골랐고 PowerShell 프로필이 예외를 던져 환경을 읽지 못함
- **THEN** 앱은 받은 환경으로 Windows의 CLI를 실행하고, 감지 결과는 그 환경에서 찾은 대로 나온다

### Requirement: FR-AGENT-017 셸 환경을 남기지 않음

앱은 셸에서 읽은 환경 변수 값을 CLI 프로세스에 넘기는 데만 써야 한다(MUST). 그 값을 디스크에 저장하지 않고, 로그, 에러 메시지, renderer로 보내는 값에 담지 않는다. 앱이 켜져 있는 동안 메모리에만 둔다.

#### Scenario: API 키가 남지 않음

- **WHEN** 셸 설정 파일에 `ANTHROPIC_API_KEY`가 있고 앱이 감지, 모델 목록 조회, 로그인 확인을 마침
- **THEN** 앱 데이터 폴더의 어떤 파일에도, renderer가 받은 어떤 명령 결과와 이벤트에도 그 키 값이 없다

#### Scenario: 실패 메시지에도 없음

- **WHEN** 셸 환경을 읽은 뒤 CLI 실행이 실패해 확인 실패 이유가 renderer로 감
- **THEN** 그 이유 문자열에 환경 변수 값이 들어 있지 않다

### Requirement: SC-AGENT-003 셸 환경 읽기 시간 상한

셸 환경을 한 번 읽는 일은 셸을 실행한 지 5초 안에 끝나야 한다(MUST). 5초 안에 끝나지 않으면 셸을 끝내고 FR-AGENT-016대로 진행하며, 감지 전체는 여전히 SC-AGENT-001의 10초 안에 끝난다.

#### Scenario: 셸 설정 파일이 멈춤

- **WHEN** 사용자의 셸 설정 파일이 입력을 기다리며 끝나지 않음
- **THEN** 셸을 실행한 지 5초 안에 앱은 그 셸을 끝내고 받은 환경으로 감지를 계속하며, 감지는 시작한 지 10초 안에 끝난다

### Requirement: FR-AGENT-018 셸 환경을 읽지 못함 표시

앱은 macOS나 Windows에서 셸 환경을 읽지 못했을 때(FR-AGENT-016) 그 사실과 이유를 감지 결과에 담고, 시작 설정 창의 감지 단계에 보여 주어야 한다(SHALL). 문구는 셸 설정 파일의 환경 변수가 반영되지 않았다는 안내이고, 이유는 시간 초과, 셸 실행 실패, 셸 오류 종료, 알아볼 수 없는 출력 중 하나다. 이유에는 환경 변수 값을 담지 않는다(FR-AGENT-017). 셸을 실행하지 않는 경우(Windows에서 명령 프롬프트를 고름)와 읽기에 성공한 경우에는 보이지 않는다.

#### Scenario: 셸 설정이 시간 안에 끝나지 않음

- **WHEN** macOS 사용자의 `.zshrc`가 5초 안에 끝나지 않아 앱이 받은 환경으로 감지함
- **THEN** 감지 단계에 셸 설정 파일의 환경 변수가 반영되지 않았다는 안내와 이유 시간 초과가 감지 결과 목록과 함께 보인다

#### Scenario: 다시 감지에서 성공

- **WHEN** 안내가 보이는 상태에서 사용자가 `.zshrc`를 고치고 다시 감지를 누르고 이번에는 셸 환경을 읽음
- **THEN** 다시 감지가 끝나면 안내가 사라진다

#### Scenario: 명령 프롬프트

- **WHEN** Windows에서 셸로 명령 프롬프트를 골랐음
- **THEN** 셸 환경을 읽지 못함 안내는 보이지 않는다
