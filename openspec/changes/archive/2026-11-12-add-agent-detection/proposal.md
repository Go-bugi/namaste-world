# Proposal

## Why

앱은 Claude Code와 Codex의 훅으로 신호를 받고(`design/mvp-feature-decisions.md` 1절), 나중에는 설치된 에이전트로 로그인하고 질문도 보냅니다. 그러려면 먼저 사용자 PC의 어디에 어떤 에이전트가 설치되어 있는지 알아야 합니다. Windows에서는 에이전트가 Windows에 바로 설치되어 있을 수도 있고 WSL 배포판 안에 설치되어 있을 수도 있어서, 설치 여부만으로는 부족하고 실행 위치까지 알아야 합니다.

## What Changes

- 앱이 로컬에 설치된 코딩 에이전트를 감지합니다. 지원 대상은 Claude Code(`claude`)와 Codex CLI(`codex`) 두 가지입니다.
- 지원 OS는 macOS와 Windows입니다. Windows에서는 Windows 자체와 설치된 WSL 배포판을 모두 확인합니다.
- 감지 결과에는 에이전트마다 설치 여부, 실행 환경(macOS, Windows, WSL 배포판 이름), 실행 파일 경로, 버전이 들어갑니다. 같은 에이전트가 여러 환경에 있으면 환경마다 따로 나옵니다.
  - 실행 환경과 경로는 이후 기능(훅 설치, 로그인, 질문 보내기)이 그 에이전트를 실행하고 설정 폴더를 찾는 데 씁니다. 그래서 감지 결과를 이 기능들이 그대로 받아 쓸 수 있는 모양으로 정합니다.
- 감지가 느리거나 실패해도(WSL이 꺼져 있음, 실행 파일이 응답하지 않음) 앱이 멈추지 않고, 정해진 시간 안에 결과를 냅니다.
- renderer가 감지 결과를 받는 Rust 명령을 추가합니다.
- 앱 시작 때 한 번 감지해 결과를 보관하고, 다시 감지를 요청하면 새로 감지합니다.
- 감지 결과를 목록으로 보여 주고 다시 감지할 수 있는 간단한 화면을 추가합니다.
- 범위 밖: 로그인, 질문 보내기, 훅 설치는 이번 변경에서 하지 않습니다.

## Capabilities

### New Capabilities

- `agent`: 로컬에 설치된 코딩 에이전트(Claude Code, Codex)를 실행 환경별로 감지하고 그 결과를 renderer에 넘깁니다.

### Modified Capabilities

없음.

## Impact

- `src-tauri`
  - 로직 모듈 `agent/`(감지 결과 해석과 판정은 `agent/domain/`, 프로세스 실행은 그 밖)
  - `commands/`에 감지 명령 추가, `lib.rs`의 `generate_handler!` 등록
  - `platform/`에 OS별 코드(Windows의 WSL 배포판 목록과 배포판 안 명령 실행, macOS의 로그인 셸 `PATH`)
  - 결과 타입에 ts-rs derive. `types/bindings/`가 새로 생깁니다
- `src/renderer`: `api/`에 감지 명령을 감싸는 함수, 감지 결과 목록 화면(`features/`)
- 의존성: 새 크레이트가 필요한지는 plan에서 정합니다.
- 참고: Orca, Paseo 같은 앱이 비슷한 감지 로직을 쓰는 것으로 보입니다. plan에서 그 방식(탐색 경로, WSL 처리, 버전 확인 방법)을 확인하고 고른 이유를 적습니다.
