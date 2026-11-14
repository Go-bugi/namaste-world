# Spec Delta

## Purpose

Claude Code와 Codex에 앱의 훅을 설치하고, 훅이 보낸 이벤트를 앱의 로컬 서버로 받아 화면에 넘깁니다. 캐릭터 반응과 성장 같은 이후 기능이 이 이벤트를 신호로 씁니다.

## ADDED Requirements

### Requirement: FR-HOOK-001 로컬 이벤트 서버

앱은 켜져 있는 동안 `127.0.0.1`에서만 접속할 수 있는 HTTP 서버를 열어 훅 이벤트를 받아야 한다(SHALL). 서버는 앱이 만든 토큰이 맞는 요청만 이벤트로 받고, 토큰이 없거나 틀린 요청은 거부한다. 앱이 꺼져 있는 동안 보낸 이벤트는 받지 않으며 나중에 몰아서 반영하지 않는다.

#### Scenario: 토큰이 맞는 요청

- **WHEN** 훅이 앱의 토큰을 넣어 이벤트를 보냄
- **THEN** 서버가 이벤트를 받고, 그 이벤트가 FR-HOOK-009에 따라 renderer로 전달된다

#### Scenario: 토큰이 틀린 요청

- **WHEN** 다른 로컬 프로세스가 토큰 없이, 또는 다른 토큰으로 이벤트를 보냄
- **THEN** 서버는 요청을 거부하고 그 이벤트는 renderer로 전달되지 않는다

#### Scenario: 외부 접속

- **WHEN** 같은 네트워크의 다른 기기가 앱이 연 포트로 접속을 시도함
- **THEN** 접속되지 않는다

### Requirement: FR-HOOK-002 실행 시 훅 설치

앱은 실행될 때 에이전트 감지(FR-AGENT-007)가 설치됨으로 낸 Claude Code와 Codex 항목마다, 그 실행 환경에 앱의 훅이 설치되어 있는지 확인해야 한다(SHALL). 지금 앱이 설치할 훅과 같으면 아무것도 바꾸지 않고, 없거나 다르면 설치한다. 설치되지 않음, 확인 실패, 멈춤인 항목에는 설치하지 않는다. 사용자가 다시 감지를 요청해 새로 설치됨으로 나온 항목에는 그 감지가 끝난 뒤 같은 방법으로 확인하고 설치한다. 훅은 사용자 전역 설정에만 설치하고, 프로젝트별 설정은 읽거나 바꾸지 않는다.

#### Scenario: 처음 실행

- **WHEN** macOS에 Claude Code와 Codex가 설치되어 있고 앱의 훅이 어디에도 없는 상태에서 앱을 실행함
- **THEN** 두 에이전트 모두에 앱의 훅이 설치된다

#### Scenario: 이미 설치됨

- **WHEN** 앱의 훅이 이미 설치되어 있는 상태에서 앱을 다시 실행함
- **THEN** 두 에이전트의 설정 파일 내용이 바뀌지 않는다

#### Scenario: 낡은 훅

- **WHEN** 이전 버전 앱이 설치한 훅이 남아 있고 지금 버전의 훅과 다름
- **THEN** 앱의 이전 훅 항목이 지금 버전의 항목으로 바뀌고, 앱의 항목이 중복으로 남지 않는다

#### Scenario: 프로젝트별 설정

- **WHEN** 어떤 프로젝트 폴더에 `.claude/settings.json`이나 `.codex/hooks.json`이 있음
- **THEN** 앱은 그 파일을 바꾸지 않고 사용자 전역 설정에만 훅을 설치한다

#### Scenario: 설치되지 않은 에이전트

- **WHEN** Codex가 설치되지 않음으로 감지됨
- **THEN** Codex 설정 파일과 폴더를 만들거나 고치지 않는다

#### Scenario: 다시 감지로 설치됨이 된 항목

- **WHEN** 앱 시작 때 WSL 배포판 `Ubuntu`가 멈춤이었고, 사용자가 다시 감지를 요청해 `Ubuntu`의 Codex가 설치됨으로 나옴
- **THEN** 앱을 다시 실행하지 않아도 `Ubuntu`의 Codex에 훅이 설치된다

#### Scenario: WSL 환경

- **WHEN** Windows에서 Claude Code가 WSL 배포판 `Ubuntu` 안에 설치됨으로 감지됨
- **THEN** 훅은 그 배포판 안의 Claude Code 설정에 설치된다

### Requirement: FR-HOOK-003 모든 훅 이벤트 등록

앱은 설치할 때 그 에이전트가 제공하는 훅 이벤트 전부에 앱의 훅을 등록해야 한다(SHALL). 도구 이름으로 거르는 이벤트는 모든 도구에 대해 실행되도록 등록한다. 다만 등록만 해도 에이전트의 기본 동작을 훅이 대신하게 되는 이벤트(Claude Code의 `WorktreeCreate`, `WorktreeRemove`)는 FR-HOOK-005를 지키기 위해 등록하지 않는다.

#### Scenario: Claude Code 이벤트

- **WHEN** Claude Code에 훅을 설치함
- **THEN** Claude Code가 제공하는 훅 이벤트(`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `Stop` 등) 각각에 앱의 훅이 하나씩 등록된다

#### Scenario: 기본 동작을 대신하는 이벤트

- **WHEN** Claude Code에 훅을 설치한 뒤 사용자가 `--worktree`로 Claude Code를 실행함
- **THEN** `WorktreeCreate`와 `WorktreeRemove`에는 앱의 훅이 없고, worktree는 앱의 훅이 없을 때와 같이 만들어지고 지워진다

#### Scenario: Codex 이벤트

- **WHEN** Codex에 훅을 설치함
- **THEN** Codex가 제공하는 훅 이벤트 각각에 앱의 훅이 하나씩 등록된다

### Requirement: FR-HOOK-004 기존 훅 보존

앱은 설치할 때 앱이 넣지 않은 훅 항목과 설정 값을 지우거나 고치거나 순서를 바꾸지 않아야 한다(MUST). 앱은 자기 항목을 다른 항목과 구별할 수 있게 넣는다. 설정 파일을 해석할 수 없으면 그 파일을 덮어쓰지 않고 설치를 실패로 남긴다.

#### Scenario: 다른 도구의 훅

- **WHEN** `~/.claude/settings.json`의 `Stop`에 사용자가 넣은 훅과 다른 도구(예: Orca)의 훅이 있음
- **THEN** 설치 뒤에도 두 훅이 같은 내용과 순서로 남고, 앱의 훅이 따로 추가된다

#### Scenario: 훅이 아닌 설정

- **WHEN** 설정 파일에 훅 외의 설정(권한, 모델 등)이 있음
- **THEN** 설치 뒤에도 그 설정의 값이 그대로다

#### Scenario: 손상된 설정 파일

- **WHEN** `~/.claude/settings.json`이 올바른 JSON이 아님
- **THEN** 앱은 그 파일을 바꾸지 않고, Claude Code의 설치 결과를 실패와 이유로 남긴다

### Requirement: FR-HOOK-005 에이전트를 방해하지 않는 훅

앱의 훅은 어떤 경우에도 성공으로 끝나야 하고(MUST), 에이전트의 동작(도구 실행 허용이나 차단, 권한 요청 응답, 프롬프트 내용)을 바꾸는 출력을 내지 않는다. 앱이 꺼져 있거나 서버가 응답하지 않거나 스크립트가 없어져도 같다. 훅이 받은 입력은 끝까지 읽어서 에이전트가 쓰기 실패를 겪지 않게 한다.

#### Scenario: 앱이 꺼져 있음

- **WHEN** 앱이 꺼져 있는 동안 Claude Code가 도구를 실행함
- **THEN** 훅은 성공으로 끝나고, 도구 실행과 다른 훅의 결과는 앱의 훅이 없을 때와 같다

#### Scenario: 권한 요청

- **WHEN** Codex가 권한 요청 훅을 실행함
- **THEN** 앱의 훅은 허용이나 거부를 결정하는 출력을 내지 않고, 사용자가 평소처럼 응답한다

#### Scenario: 스크립트가 지워짐

- **WHEN** 사용자가 앱 데이터 폴더의 훅 스크립트를 지운 뒤 Claude Code를 사용함
- **THEN** 훅은 오류 없이 성공으로 끝나고 Claude Code는 평소처럼 동작한다

### Requirement: FR-HOOK-006 권한 문제 처리

앱은 훅이 동작하는 데 필요한 권한과 설정을 설치할 때 함께 갖춰야 한다(SHALL): 훅 스크립트의 실행 권한, 없는 설정 폴더와 파일 생성, Codex의 훅 기능 켜기와 앱 훅의 신뢰 승인. 앱이 갖출 수 없는 경우(쓰기 권한이 없는 파일 등)는 그 에이전트와 실행 환경의 설치를 실패로 남기고 이유를 함께 담는다.

#### Scenario: 스크립트 실행 권한

- **WHEN** 앱이 훅 스크립트를 설치함
- **THEN** 스크립트는 그 실행 환경의 사용자가 실행할 수 있고, 다른 사용자는 고칠 수 없다

#### Scenario: 설정 파일이 없음

- **WHEN** Claude Code는 설치되어 있지만 `~/.claude/settings.json`이 없음
- **THEN** 앱이 파일을 만들고 훅을 설치한다

#### Scenario: Codex 훅 기능이 꺼져 있음

- **WHEN** Codex 설정에 훅 기능이 켜져 있지 않음
- **THEN** 앱이 훅 기능을 켜고 훅을 설치하며, 사용자가 별도로 승인하지 않아도 Codex가 앱의 훅을 실행한다

#### Scenario: 사용자가 Codex 훅 기능을 꺼 둠

- **WHEN** 사용자가 Codex 설정에 훅 기능을 끈다고 직접 적어 둠
- **THEN** 앱은 그 설정과 Codex 훅 파일을 바꾸지 않고, Codex의 설치 결과를 실패와 이유로 남긴다

#### Scenario: 쓸 수 없는 설정 파일

- **WHEN** `~/.codex/hooks.json`에 쓰기 권한이 없음
- **THEN** 앱은 파일 권한을 바꾸지 않고, Codex의 설치 결과를 실패와 이유로 남긴다

### Requirement: FR-HOOK-007 설치 결과 보관

앱은 설치를 확인한 결과를 에이전트와 실행 환경마다 새로 설치함, 이미 설치됨, 실패(이유 포함) 가운데 하나로 앱이 켜져 있는 동안 보관하고, renderer가 요청하면 주어야 한다(SHALL). 한 항목의 실패는 다른 항목의 설치를 막지 않는다.

#### Scenario: 일부 실패

- **WHEN** Claude Code 설치는 성공하고 Codex 설정 파일에 쓸 수 없음
- **THEN** 결과에 Claude Code는 새로 설치함, Codex는 실패와 이유가 나오고, Claude Code의 훅은 정상 동작한다

### Requirement: FR-HOOK-008 포트가 바뀌어도 전달

훅은 앱이 실행될 때마다 서버 포트나 토큰이 바뀌어도 설정 파일을 다시 고치지 않고 지금 켜진 앱의 서버로 이벤트를 보내야 한다(SHALL).

#### Scenario: 앱 재시작

- **WHEN** 앱을 종료했다가 다시 실행해 서버 포트가 바뀜
- **THEN** 설정 파일은 바뀌지 않고, 이후 훅 이벤트는 새 서버로 전달된다

#### Scenario: WSL에서 보낸 이벤트

- **WHEN** WSL 배포판 `Ubuntu` 안의 Codex가 훅을 실행함
- **THEN** 이벤트가 Windows에서 켜진 앱의 서버로 전달된다

### Requirement: FR-HOOK-009 renderer로 이벤트 전달

앱은 받은 훅 이벤트마다 어느 에이전트에서 왔는지, 이벤트 이름, 받은 시각, 세션 ID(있으면)와 훅이 보낸 원래 입력을 renderer에 넘겨야 한다(SHALL).

#### Scenario: Claude Code 이벤트

- **WHEN** Claude Code에서 `UserPromptSubmit` 훅이 실행됨
- **THEN** renderer가 에이전트 Claude Code, 이벤트 `UserPromptSubmit`, 받은 시각, 세션 ID가 담긴 이벤트를 받는다

#### Scenario: 해석할 수 없는 입력

- **WHEN** 토큰은 맞지만 본문이 JSON이 아닌 요청이 옴
- **THEN** 서버는 앱을 멈추지 않고 그 요청을 무시한다

### Requirement: FR-HOOK-012 이벤트 내용을 디스크에 남기지 않음

앱은 받은 훅 이벤트의 내용(프롬프트, 파일 경로, 도구 입력과 출력 등 훅이 보낸 입력)을 파일, 로그, DB 등 디스크에 남기지 않아야 한다(MUST). 이벤트는 앱이 켜져 있는 동안 메모리에만 두고 앱을 종료하면 사라진다.

#### Scenario: 앱 종료 뒤

- **WHEN** 이벤트를 받은 뒤 앱을 종료함
- **THEN** 앱 데이터 폴더와 로그 어디에도 그 이벤트의 프롬프트나 도구 입력, 출력이 남아 있지 않다

### Requirement: FR-HOOK-013 큰 요청 본문

앱은 본문이 1MB를 넘는 훅 요청을 받으면 원래 입력 전체 대신 정해진 필드만 남겨 renderer에 넘겨야 한다(SHALL). 정해진 필드는 에이전트, 이벤트 이름, 받은 시각, 세션 ID, 도구 이름, 도구 호출 ID, 에러 여부, 작업 폴더(cwd) 가운데 그 요청에 있는 것이고, 원래 입력이 잘렸다는 표시를 함께 넘긴다. 1MB 이하인 요청은 FR-HOOK-009대로 원래 입력 전체를 넘긴다.

#### Scenario: 큰 도구 출력

- **WHEN** Claude Code의 `PostToolUse` 훅이 도구 출력 때문에 3MB인 본문을 보냄
- **THEN** renderer는 에이전트, 이벤트 이름 `PostToolUse`, 세션 ID, 도구 이름, 도구 호출 ID가 담기고 원래 입력은 잘렸다고 표시된 이벤트를 받는다

#### Scenario: 1MB 이하

- **WHEN** 본문이 200KB인 훅 요청이 옴
- **THEN** renderer는 원래 입력 전체가 담긴 이벤트를 받는다

### Requirement: FR-HOOK-014 앱은 하나만 실행

앱은 한 사용자 세션에서 하나만 실행되어야 한다(SHALL). 이미 켜져 있을 때 다시 실행하면 새 앱을 띄우지 않고 켜져 있는 앱의 `main` 창을 앞으로 가져오며, 훅 이벤트는 처음 켜진 앱이 계속 받는다.

#### Scenario: 두 번 실행

- **WHEN** 앱이 켜져 있는 상태에서 사용자가 앱을 한 번 더 실행함
- **THEN** 앱 프로세스는 하나로 남고, 켜져 있던 앱의 `main` 창이 앞으로 나오며, 이후 훅 이벤트도 그 앱 화면에 나온다

### Requirement: FR-HOOK-010 훅 이벤트 목록 화면

앱은 `main` 창의 한 영역에 이번 실행에서 받은 훅 이벤트 중 최근 200개를 받은 순서대로 보여 주어야 한다(SHALL). 200개를 넘으면 가장 오래된 이벤트부터 목록에서 빠진다. 각 줄에는 받은 시각, 에이전트, 이벤트 이름을 보여 준다. 화면에는 FR-HOOK-007의 설치 결과도 에이전트와 실행 환경마다 보여 준다.

#### Scenario: 질문하면 이벤트가 보임

- **WHEN** 앱이 켜진 상태에서 터미널의 Claude Code에 질문함
- **THEN** 화면 목록에 Claude Code의 `UserPromptSubmit`부터 `Stop`까지 이벤트가 받은 순서대로 추가된다

#### Scenario: 200개를 넘음

- **WHEN** 이번 실행에서 이벤트를 201개 받음
- **THEN** 목록에는 두 번째로 받은 이벤트부터 마지막 이벤트까지 200개가 보인다

#### Scenario: 설치 실패 표시

- **WHEN** Codex 설치가 실패함
- **THEN** 화면에 Codex와 그 실행 환경, 실패 이유가 보인다

### Requirement: FR-HOOK-011 macOS와 Windows 지원

앱은 macOS와 Windows에서 FR-HOOK-001~010을 모두 지켜야 한다(SHALL). Windows에서는 Windows에 바로 설치된 에이전트와 WSL 배포판 안의 에이전트를 각각 별도 실행 환경으로 다루고, 각 환경의 설정 위치에 훅을 설치한다. 훅이 동작하는 데 에이전트 자체를 실행하는 데 필요한 것 외에 사용자가 따로 설치할 프로그램이 없어야 한다.

#### Scenario: Windows에 바로 설치된 에이전트

- **WHEN** Windows에 Claude Code와 Codex가 설치되어 있고 앱을 실행함
- **THEN** 두 에이전트의 Windows 사용자 설정(`%USERPROFILE%\.claude`, `%USERPROFILE%\.codex`)에 훅이 설치되고, 각 에이전트에 질문하면 이벤트가 앱 화면에 나온다

#### Scenario: Windows와 WSL에 모두 설치됨

- **WHEN** Codex가 Windows에도 있고 WSL 배포판 `Ubuntu` 안에도 있음
- **THEN** 두 환경 각각에 훅이 설치되고, 설치 결과도 환경마다 따로 나온다

#### Scenario: Windows 경로에 공백이나 한글

- **WHEN** Windows 사용자 폴더 경로에 공백이나 한글이 들어 있음(예: `C:\Users\홍 길동`)
- **THEN** 훅이 설치되고 이벤트가 앱으로 전달된다

#### Scenario: Windows에서 앱이 꺼져 있음

- **WHEN** Windows에서 앱이 꺼져 있는 동안 Codex가 훅을 실행함
- **THEN** 훅은 SC-HOOK-001의 시간 안에 성공으로 끝나고, 콘솔 창이 새로 뜨지 않는다

### Requirement: SC-HOOK-001 훅 실행 시간 상한

앱의 훅 한 번은 앱이 꺼져 있거나 응답하지 않아도 시작한 지 2초 안에 끝나야 한다(MUST).

#### Scenario: 서버가 응답하지 않음

- **WHEN** 앱의 서버가 연결은 받았지만 응답하지 않음
- **THEN** 훅은 시작한 지 2초 안에 성공으로 끝난다

### Requirement: SC-HOOK-002 이벤트 표시 지연

앱이 켜져 있을 때 훅이 실행되면 그 이벤트는 1초 안에 화면 목록에 나타나야 한다(MUST).

#### Scenario: 이벤트 표시

- **WHEN** 앱이 켜진 상태에서 Codex가 `UserPromptSubmit` 훅을 실행함
- **THEN** 훅이 시작한 지 1초 안에 화면 목록에 그 이벤트가 나타난다
