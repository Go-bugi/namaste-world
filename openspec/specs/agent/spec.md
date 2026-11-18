# agent Specification

## Purpose
사용자 PC에 설치된 코딩 에이전트(Claude Code, Codex)를 실행 환경별로 찾아, 훅 설치와 로그인, 질문 보내기 같은 이후 기능이 그 에이전트를 실행할 수 있게 합니다.

## Requirements

### Requirement: FR-AGENT-001 지원 에이전트 감지

앱은 Claude Code(`claude`)와 Codex CLI(`codex`)가 설치되어 있는지 감지해야 한다(SHALL). 감지 시점과 결과를 주는 방식은 FR-AGENT-007을 따른다. 두 에이전트 외의 프로그램은 감지 대상이 아니다.

#### Scenario: 둘 다 설치됨

- **WHEN** macOS에 `claude`와 `codex`가 모두 설치되어 있고 감지가 끝남
- **THEN** 결과에 Claude Code와 Codex가 각각 설치됨으로 나온다

#### Scenario: 하나만 설치됨

- **WHEN** `claude`만 설치되어 있고 `codex`는 어느 환경에도 없음
- **THEN** 결과에 Claude Code는 설치됨, Codex는 설치되지 않음으로 나온다

#### Scenario: 설치 방법과 무관

- **WHEN** 에이전트가 공식 설치 스크립트, npm 전역 설치, Homebrew, winget 가운데 어느 방법으로 설치되어 있음
- **THEN** 설치 방법과 상관없이 설치됨으로 나온다

### Requirement: FR-AGENT-002 실행 환경별 결과

앱은 감지 결과를 실행 환경마다 따로 돌려주어야 한다(SHALL). 실행 환경은 macOS에서는 macOS 하나이고, Windows에서는 Windows 자체와 설치된 WSL 배포판 각각이다. 멈춘(Stopped) 배포판은 사용자가 다시 감지를 요청했을 때만 켜서 확인하고, 앱 시작 감지에서는 켜지 않고 그 배포판의 두 에이전트를 "멈춤"으로 낸다. Docker Desktop처럼 도구가 쓰는 배포판(`docker-desktop`, `docker-desktop-data`)은 확인하지 않는다. 같은 에이전트가 여러 환경에 있으면 환경마다 별도 항목으로 나온다.

#### Scenario: Windows와 WSL에 모두 설치됨

- **WHEN** Windows에 `codex`가 있고 WSL 배포판 `Ubuntu` 안에도 `codex`가 있음
- **THEN** Codex 항목이 두 개 나오고, 하나는 실행 환경이 Windows, 다른 하나는 WSL `Ubuntu`다

#### Scenario: WSL 배포판이 여럿

- **WHEN** WSL 배포판 `Ubuntu`와 `Debian`이 설치되어 있고 `claude`는 `Debian`에만 있음
- **THEN** Claude Code 항목은 실행 환경이 WSL `Debian`인 것 하나만 설치됨으로 나온다

#### Scenario: 앱 시작 감지에서 멈춘 배포판

- **WHEN** 앱 시작 감지 때 WSL 배포판 `Ubuntu`가 Stopped 상태임
- **THEN** 앱은 `Ubuntu`를 켜지 않고, WSL `Ubuntu` 환경의 Claude Code와 Codex를 멈춤으로 낸다

#### Scenario: 다시 감지에서 멈춘 배포판

- **WHEN** 사용자가 다시 감지를 요청했고 WSL 배포판 `Ubuntu`가 Stopped 상태이며 그 안에 `codex`가 있음
- **THEN** 앱이 `Ubuntu`를 켜서 확인하고, Codex가 WSL `Ubuntu` 환경에 설치됨으로 나온다

#### Scenario: 도구용 배포판

- **WHEN** WSL 배포판 목록에 `Ubuntu`와 `docker-desktop`이 있음
- **THEN** `Ubuntu`만 실행 환경으로 나오고 `docker-desktop`은 결과에 나오지 않는다

#### Scenario: WSL이 없음

- **WHEN** Windows에 WSL이 설치되어 있지 않거나 배포판이 하나도 없음
- **THEN** Windows 환경의 결과만 나오고, 에러로 처리하지 않는다

### Requirement: FR-AGENT-003 설치된 항목의 내용

설치됨으로 나온 항목은 실행 환경, 그 환경에서 에이전트를 실행할 수 있는 실행 파일 경로, 버전 문자열을 담아야 한다(SHALL). WSL 항목의 경로는 그 배포판 안의 경로이고 배포판 이름이 함께 나온다. 이후 기능은 이 정보만으로 같은 에이전트를 실행할 수 있어야 한다.

#### Scenario: WSL 항목

- **WHEN** WSL 배포판 `Ubuntu`의 `/home/user/.local/bin/claude`에 Claude Code가 있음
- **THEN** 항목에 실행 환경 WSL, 배포판 이름 `Ubuntu`, 경로 `/home/user/.local/bin/claude`, 그 실행 파일이 알려 준 버전이 담긴다

#### Scenario: 버전을 읽지 못함

- **WHEN** 실행 파일은 찾았지만 버전 확인 명령이 실패하거나 시간 안에 응답하지 않음
- **THEN** 항목은 설치됨이 아니라 확인 실패로 나오고, 이유는 버전 확인 실패다

### Requirement: FR-AGENT-004 macOS 로그인 셸의 PATH 반영

macOS에서 앱은 Finder나 Dock으로 실행되어 셸의 `PATH`를 물려받지 못해도, 사용자의 로그인 셸에서 찾을 수 있는 에이전트를 감지해야 한다(MUST).

#### Scenario: 셸 설정으로만 PATH에 들어간 에이전트

- **WHEN** `claude`가 사용자의 셸 설정 파일이 추가한 디렉터리(예: `~/.local/bin`)에만 있고, 앱은 Finder에서 실행됨
- **THEN** Claude Code가 설치됨으로 나온다

### Requirement: FR-AGENT-005 확인 실패와 미설치 구분

앱은 어떤 실행 환경을 확인하지 못했을 때 그 환경의 에이전트를 "설치되지 않음"이 아니라 "확인 실패"로 돌려주고, 실패 이유를 함께 담아야 한다(SHALL). 한 환경의 실패는 다른 환경의 결과에 영향을 주지 않는다.

#### Scenario: WSL 배포판이 응답하지 않음

- **WHEN** WSL 배포판 `Ubuntu` 안에서 명령이 정해진 시간 안에 끝나지 않음
- **THEN** `Ubuntu` 환경의 Claude Code와 Codex는 확인 실패로 나오고, Windows 환경의 결과는 정상으로 나온다

#### Scenario: 실행 파일이 없음

- **WHEN** 환경은 정상적으로 확인했고 그 안에 `codex`가 없음
- **THEN** 그 환경의 Codex는 확인 실패가 아니라 설치되지 않음으로 나온다

### Requirement: FR-AGENT-006 감지 중에도 앱이 멈추지 않음

감지는 앱의 화면과 다른 명령을 막지 않아야 한다(MUST). renderer는 감지 결과를 비동기로 받는다.

#### Scenario: 느린 감지

- **WHEN** 감지가 진행 중임
- **THEN** 창 이동, 다른 명령 호출, 3D 화면 갱신이 감지가 끝나기를 기다리지 않고 동작한다

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

### Requirement: FR-AGENT-008 감지 결과 목록 화면

앱은 시작 설정 창의 감지 단계에 감지 결과를 에이전트와 실행 환경마다 한 줄씩 보여 주는 목록을 제공해야 한다(SHALL). 각 줄에는 에이전트 이름, 실행 환경, 상태(설치됨, 설치되지 않음, 확인 실패, 멈춤)를 보여 주고, 설치됨이면 버전과 경로를, 확인 실패면 이유를, 멈춤이면 다시 감지하면 배포판을 켜서 확인한다는 안내를 함께 보여 준다. 화면에는 다시 감지하는 버튼이 있고, 감지가 진행 중인 동안(앱 시작 감지 포함)과 훅 설치가 진행 중인 동안(FR-HOOK-002의 설치와 FR-HOOK-016의 다시 설치 포함) 이 버튼은 비활성화된다.

#### Scenario: 결과 표시

- **WHEN** Windows 환경에 Codex가 설치됨, WSL `Ubuntu` 환경은 확인 실패로 감지됨
- **THEN** 시작 설정 창의 감지 단계에 Windows의 Codex 줄(버전과 경로 포함)과, WSL `Ubuntu`의 Claude Code와 Codex 줄(확인 실패와 이유 포함)이 나온다

#### Scenario: 처음 감지 중

- **WHEN** 앱 시작 감지가 진행 중이고 보관한 결과가 없음
- **THEN** 화면에 확인 중임이 보이고 다시 감지 버튼은 비활성화되며, 감지가 끝나면 결과로 바뀌고 버튼이 활성화된다

#### Scenario: 다시 감지 중

- **WHEN** 이전 결과가 화면에 있고 다시 감지가 진행 중임
- **THEN** 이전 결과 목록은 그대로 보이고 확인 중 표시가 더해지며, 다시 감지 버튼은 비활성화된다

#### Scenario: 다시 감지 버튼

- **WHEN** 사용자가 다시 감지 버튼을 누름
- **THEN** FR-AGENT-007의 다시 감지가 실행되고, 끝나면 화면이 새 결과로 바뀌고 버튼이 다시 활성화된다

#### Scenario: 다시 감지 뒤 훅 설치 중

- **WHEN** 다시 감지가 끝나고 그 뒤의 훅 설치가 진행 중임
- **THEN** 다시 감지 버튼은 비활성화되고, 설치가 끝나면 다시 활성화된다

### Requirement: SC-AGENT-001 감지 시간 상한

감지 한 번은 최악의 경우에도 시작한 지 10초 안에 끝나야 한다(MUST). 시간 안에 확인하지 못한 환경은 FR-AGENT-005에 따라 확인 실패로 나온다.

#### Scenario: 모든 확인이 멈춤

- **WHEN** 모든 실행 환경의 확인 명령이 응답하지 않음
- **THEN** 감지를 시작한 지 10초 안에 결과가 나오고, 응답하지 않은 환경은 모두 확인 실패로 나온다

### Requirement: FR-AGENT-009 로그인 상태 확인

앱은 renderer가 요청한 에이전트와 실행 환경 한 쌍의 로그인 상태를, 그 실행 환경에 설치된 CLI에 물어 확인해야 한다(SHALL). CLI는 감지 결과의 실행 파일 경로(FR-AGENT-003)로 실행하고, WSL 항목은 그 배포판 안에서 실행한다. 결과는 로그인됨, 로그인 안 됨, 확인 실패 중 하나다. CLI가 알려 주는 로그인이면 방식(구독, API 키 등)과 상관없이 로그인됨이고, CLI가 로그인 방식을 알려 주면 결과에 함께 담는다.

#### Scenario: 구독으로 로그인됨

- **WHEN** macOS의 Claude Code가 claude.ai 구독으로 로그인되어 있고 renderer가 그 항목의 로그인 상태를 요청함
- **THEN** 결과는 로그인됨이고 로그인 방식이 함께 담긴다

#### Scenario: 로그인 안 됨

- **WHEN** WSL `Ubuntu`의 Codex가 로그인되어 있지 않고 renderer가 그 항목의 로그인 상태를 요청함
- **THEN** 결과는 로그인 안 됨이다

#### Scenario: API 키로 로그인됨

- **WHEN** Windows의 Codex가 API 키로 로그인되어 있음
- **THEN** 결과는 로그인됨이고 로그인 방식은 API 키다

### Requirement: FR-AGENT-010 로그인 확인 실패와 로그인 안 됨 구분

앱은 로그인 상태를 알아내지 못했을 때 결과를 "로그인 안 됨"이 아니라 "확인 실패"로 돌려주고 이유를 함께 담아야 한다(SHALL). CLI 실행 실패, 시간 초과(SC-AGENT-002), 알아볼 수 없는 출력이 확인 실패에 해당한다.

#### Scenario: CLI가 응답하지 않음

- **WHEN** 로그인 상태를 묻는 CLI가 시간 안에 끝나지 않음
- **THEN** 결과는 확인 실패이고 이유는 시간 초과다

#### Scenario: 알아볼 수 없는 출력

- **WHEN** CLI가 로그인 여부를 판단할 수 없는 출력을 내고 끝남
- **THEN** 결과는 확인 실패이고, 로그인 안 됨으로 나오지 않는다

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

### Requirement: SC-AGENT-002 로그인 확인 시간 상한

로그인 상태 확인 한 번은 시작한 지 5초 안에 결과를 내야 한다(MUST). 시간 안에 끝나지 않으면 FR-AGENT-010에 따라 확인 실패로 낸다.

#### Scenario: CLI가 멈춤

- **WHEN** 로그인 상태를 묻는 CLI가 응답하지 않음
- **THEN** 확인을 시작한 지 5초 안에 확인 실패 결과가 나온다

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
