## MODIFIED Requirements

### Requirement: FR-HOOK-002 실행 시 훅 설치

앱은 실행될 때 에이전트 감지(FR-AGENT-007)가 설치됨으로 낸 Claude Code와 Codex 항목마다, 그 실행 환경에 앱의 훅이 설치되어 있는지 확인해야 한다(SHALL). 지금 앱이 설치할 훅과 같으면 아무것도 바꾸지 않고, 없거나 다르면 설치한다. 설치되지 않음, 확인 실패, 멈춤인 항목에는 설치하지 않는다. 사용자가 다시 감지를 요청해 새로 설치됨으로 나온 항목에는 그 감지가 끝난 뒤 같은 방법으로 확인하고 설치한다. 훅은 사용자 전역 설정에만 설치하고, 프로젝트별 설정은 읽거나 바꾸지 않는다. 사용자 전역 설정 폴더는 그 실행 환경에서 CLI를 실행하는 환경(FR-AGENT-012~014)의 `CLAUDE_CONFIG_DIR`, `CODEX_HOME`이 가리키는 폴더이고, 값이 없으면 홈의 `.claude`, `.codex`다. 설정 폴더가 바뀌어 새 폴더에 설치해도 이전 폴더에 설치한 앱 훅은 그대로 둔다.

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

#### Scenario: 셸에서 바꾼 설정 폴더

- **WHEN** macOS 사용자가 `.zshrc`에 `export CLAUDE_CONFIG_DIR=~/work-claude`를 적었고 앱은 Finder에서 실행됨
- **THEN** 훅은 `~/work-claude/settings.json`에 설치되고 `~/.claude/settings.json`은 바뀌지 않는다

#### Scenario: WSL에서 바꾼 Codex 설정 폴더

- **WHEN** WSL `Ubuntu` 사용자의 `.bashrc`에 `export CODEX_HOME=~/codex-work`가 있고 Codex가 설치됨으로 감지됨
- **THEN** 훅은 그 배포판의 `~/codex-work` 안 Codex 설정에 설치된다

#### Scenario: 설정 폴더 변수가 없음

- **WHEN** Windows에서 CLI 실행 환경에 `CODEX_HOME`이 없음
- **THEN** 훅은 `%USERPROFILE%\.codex`의 Codex 설정에 설치된다

#### Scenario: 설정 폴더가 바뀜

- **WHEN** 지난 실행에서 `~/.claude/settings.json`에 앱 훅을 설치했고, 그 뒤 사용자가 `.zshrc`에 `export CLAUDE_CONFIG_DIR=~/work-claude`를 적고 앱을 다시 실행함
- **THEN** 앱 훅이 `~/work-claude/settings.json`에 설치되고, `~/.claude/settings.json`의 앱 훅은 지워지지 않고 그대로 남는다

