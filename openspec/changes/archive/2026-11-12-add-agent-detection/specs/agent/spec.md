# Spec Delta

## Purpose

사용자 PC에 설치된 코딩 에이전트(Claude Code, Codex)를 실행 환경별로 찾아, 훅 설치와 로그인, 질문 보내기 같은 이후 기능이 그 에이전트를 실행할 수 있게 합니다.

## ADDED Requirements

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

앱은 시작할 때 감지를 한 번 하고 그 결과를 앱이 켜져 있는 동안 보관해야 한다(SHALL). renderer가 결과를 요청하면 보관한 결과를 준다. renderer가 다시 감지를 요청하면 새로 감지하고 보관한 결과를 바꾼다. 결과는 디스크에 저장하지 않는다.

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

### Requirement: FR-AGENT-008 감지 결과 목록 화면

앱은 `main` 창의 한 영역에 감지 결과를 에이전트와 실행 환경마다 한 줄씩 보여 주는 목록을 제공해야 한다(SHALL). 각 줄에는 에이전트 이름, 실행 환경, 상태(설치됨, 설치되지 않음, 확인 실패, 멈춤)를 보여 주고, 설치됨이면 버전과 경로를, 확인 실패면 이유를, 멈춤이면 다시 감지하면 배포판을 켜서 확인한다는 안내를 함께 보여 준다. 화면에는 다시 감지하는 버튼이 있고, 감지가 진행 중인 동안(앱 시작 감지 포함) 이 버튼은 비활성화된다.

#### Scenario: 결과 표시

- **WHEN** Windows 환경에 Codex가 설치됨, WSL `Ubuntu` 환경은 확인 실패로 감지됨
- **THEN** 화면에 Windows의 Codex 줄(버전과 경로 포함)과, WSL `Ubuntu`의 Claude Code와 Codex 줄(확인 실패와 이유 포함)이 나온다

#### Scenario: 처음 감지 중

- **WHEN** 앱 시작 감지가 진행 중이고 보관한 결과가 없음
- **THEN** 화면에 확인 중임이 보이고 다시 감지 버튼은 비활성화되며, 감지가 끝나면 결과로 바뀌고 버튼이 활성화된다

#### Scenario: 다시 감지 중

- **WHEN** 이전 결과가 화면에 있고 다시 감지가 진행 중임
- **THEN** 이전 결과 목록은 그대로 보이고 확인 중 표시가 더해지며, 다시 감지 버튼은 비활성화된다

#### Scenario: 다시 감지 버튼

- **WHEN** 사용자가 다시 감지 버튼을 누름
- **THEN** FR-AGENT-007의 다시 감지가 실행되고, 끝나면 화면이 새 결과로 바뀌고 버튼이 다시 활성화된다

### Requirement: SC-AGENT-001 감지 시간 상한

감지 한 번은 최악의 경우에도 시작한 지 10초 안에 끝나야 한다(MUST). 시간 안에 확인하지 못한 환경은 FR-AGENT-005에 따라 확인 실패로 나온다.

#### Scenario: 모든 확인이 멈춤

- **WHEN** 모든 실행 환경의 확인 명령이 응답하지 않음
- **THEN** 감지를 시작한 지 10초 안에 결과가 나오고, 응답하지 않은 환경은 모두 확인 실패로 나온다
