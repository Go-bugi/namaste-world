# Spec Delta

## ADDED Requirements

### Requirement: FR-HOOK-015 훅 설치 결과 화면

앱은 시작 설정 창의 훅 설치 단계에 FR-HOOK-007의 설치 결과를 에이전트와 실행 환경마다 보여 주어야 한다(SHALL). 각 줄에는 에이전트 이름, 실행 환경, 성공 여부를 보여 주고, 실패면 이유를 함께 보여 준다. 설치가 진행 중이면 진행 중임을 보여 준다.

#### Scenario: 설치 실패 표시

- **WHEN** Codex 설치가 실패함
- **THEN** 훅 설치 단계에 Codex와 그 실행 환경, 실패 이유가 보인다

#### Scenario: 다시 감지 뒤 다시 설치

- **WHEN** 다시 감지 버튼을 눌러 FR-HOOK-002의 설치가 다시 실행됨
- **THEN** 훅 설치 단계는 진행 중으로 바뀌었다가 새 설치 결과로 바뀐다

## MODIFIED Requirements

### Requirement: FR-HOOK-014 앱은 하나만 실행

앱은 한 사용자 세션에서 하나만 실행되어야 한다(SHALL). 이미 켜져 있을 때 다시 실행하면 새 앱을 띄우지 않고 켜져 있는 앱의 창을 FR-WINDOW-006대로 앞으로 가져오며, 훅 이벤트는 처음 켜진 앱이 계속 받는다.

#### Scenario: 두 번 실행

- **WHEN** 앱이 켜져 있고 작은 창으로 넘어간 상태에서 사용자가 앱을 한 번 더 실행함
- **THEN** 앱 프로세스는 하나로 남고, 켜져 있던 앱의 작은 창이 앞으로 나오며, 이후 훅 이벤트도 그 앱이 받는다

#### Scenario: 시작 설정 중 두 번 실행

- **WHEN** 시작 설정 창이 열려 있는 상태에서 사용자가 앱을 한 번 더 실행함
- **THEN** 앱 프로세스는 하나로 남고 시작 설정 창이 앞으로 나온다

### Requirement: FR-HOOK-011 macOS와 Windows 지원

앱은 macOS와 Windows에서 FR-HOOK-001~009, FR-HOOK-012~015를 모두 지켜야 한다(SHALL). Windows에서는 Windows에 바로 설치된 에이전트와 WSL 배포판 안의 에이전트를 각각 별도 실행 환경으로 다루고, 각 환경의 설정 위치에 훅을 설치한다. 훅이 동작하는 데 에이전트 자체를 실행하는 데 필요한 것 외에 사용자가 따로 설치할 프로그램이 없어야 한다.

#### Scenario: Windows에 바로 설치된 에이전트

- **WHEN** Windows에 Claude Code와 Codex가 설치되어 있고 앱을 실행함
- **THEN** 두 에이전트의 Windows 사용자 설정(`%USERPROFILE%\.claude`, `%USERPROFILE%\.codex`)에 훅이 설치되고, 각 에이전트에 질문하면 이벤트가 renderer에 전달된다

#### Scenario: Windows와 WSL에 모두 설치됨

- **WHEN** Codex가 Windows에도 있고 WSL 배포판 `Ubuntu` 안에도 있음
- **THEN** 두 환경 각각에 훅이 설치되고, 설치 결과도 환경마다 따로 나온다

#### Scenario: Windows 경로에 공백이나 한글

- **WHEN** Windows 사용자 폴더 경로에 공백이나 한글이 들어 있음(예: `C:\Users\홍 길동`)
- **THEN** 훅이 설치되고 이벤트가 앱으로 전달된다

#### Scenario: Windows에서 앱이 꺼져 있음

- **WHEN** Windows에서 앱이 꺼져 있는 동안 Codex가 훅을 실행함
- **THEN** 훅은 SC-HOOK-001의 시간 안에 성공으로 끝나고, 콘솔 창이 새로 뜨지 않는다

## REMOVED Requirements

### Requirement: FR-HOOK-010 훅 이벤트 목록 화면

**Reason**: 시작 설정 창에는 받은 훅 이벤트 목록을 두지 않습니다. 이벤트는 FR-HOOK-009대로 계속 renderer에 전달되고, 이후 캐릭터 반응 같은 기능이 신호로 씁니다. 설치 결과 표시는 FR-HOOK-015로 옮깁니다.

**Migration**: 이벤트 목록 화면을 쓰던 사용자는 없습니다. 이벤트 수신 확인은 개발자 도구에서 renderer 이벤트를 보거나 이후 기능의 화면으로 합니다.

### Requirement: SC-HOOK-002 이벤트 표시 지연

**Reason**: 이벤트를 보여 주는 화면 목록이 없어져 측정 대상이 없습니다.

**Migration**: 이벤트를 화면에 보여 주는 기능이 다시 생기면 그 기능의 스펙에 표시 지연 기준을 새 ID로 둡니다.
