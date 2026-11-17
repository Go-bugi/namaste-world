# Proposal

## Why

앱이 에이전트 CLI를 실행하는 환경은 사용자가 터미널에서 CLI를 쓰는 환경과 다릅니다. 셸 설정 파일에만 적은 환경 변수(`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` 등)가 앱이 실행한 CLI에 전달되지 않습니다. 그래서 터미널에서는 로그인되어 있는데 시작 설정 창의 로그인 확인 단계에는 "로그인 안 됨"으로 나옵니다(이슈 #21). 로그인 확인 단계가 생기면서 이 차이가 화면에 드러났고, 이후 질문 보내기도 같은 환경을 쓰므로 실제로 실패합니다.

## What Changes

- **macOS**: 로그인 셸(`$SHELL -ilc`)에서 지금은 `PATH`만 읽습니다. 이를 환경 전체로 넓혀 감지, 모델 목록 조회, 로그인 확인에 넘깁니다. 변수를 골라 넘기면 `ANTHROPIC_BASE_URL`, 프록시, 인증서 같은 다른 변수에서 같은 문제가 다시 생기므로 전체를 넘깁니다.
- **WSL**: 지금은 `sh -lc`로 실행해서 `.profile`만 읽고 `.bashrc`, `.zshrc`는 읽지 않습니다. 이를 배포판 사용자의 로그인 셸로 실행하도록 바꿉니다. bash, zsh는 대화형 로그인 셸로 실행합니다.
- **Windows 자체**: Windows에는 사용자의 셸을 알려 주는 값이 없어서, 시작 설정 창에 셸을 고르는 단계를 Windows에서만 추가합니다.
  - 선택지는 PowerShell, 명령 프롬프트, Git Bash(설치된 경우)입니다.
  - 기본값은 PowerShell입니다.
  - 고른 셸의 설정 파일을 읽은 환경으로 Windows의 CLI를 실행합니다. 명령 프롬프트를 고르면 앱이 받은 환경을 그대로 씁니다.
  - 선택은 앱 데이터 폴더에 보관해 다음 실행 때도 씁니다.
  - WSL 환경은 위의 WSL 방식을 따르므로 이 선택과 상관없습니다.
- **설정 폴더**: 앱이 CLI 설정 폴더를 직접 다루는 곳(훅 설치, Codex 설정 파일 읽기)은 지금 `~/.claude`, `~/.codex`로 고정되어 있습니다. 이를 세 환경 모두에서 CLI 실행에 쓰는 환경의 `CLAUDE_CONFIG_DIR`, `CODEX_HOME`을 따르도록 바꿉니다. 그래야 훅이 CLI가 실제로 읽는 설정에 설치됩니다.
- **인증 값**: 셸에서 읽은 환경(API 키, 토큰 포함)은 CLI 프로세스에 넘기기만 합니다. 디스크에 저장하지 않고, 로그, 에러 메시지, renderer로 보내는 값에도 담지 않습니다(FR-AGENT-011과 같은 원칙). 디스크에 보관하는 것은 셸 선택뿐입니다.

## Capabilities

### New Capabilities

없음.

### Modified Capabilities

- `agent`:
  - FR-AGENT-004(macOS 로그인 셸의 `PATH`)를 환경 전체로 넓힙니다.
  - WSL에서 로그인 셸 설정을 읽는 요구사항과, Windows에서 고른 셸의 환경으로 CLI를 실행하는 요구사항을 추가합니다.
  - 셸 환경을 저장하거나 노출하지 않는 요구사항을 추가합니다.
- `hook`: FR-HOOK-002의 "사용자 전역 설정" 위치가 CLI 실행 환경의 `CLAUDE_CONFIG_DIR`, `CODEX_HOME`을 따르도록 바꿉니다.
- `start`:
  - Windows에서 셸 선택 단계를 추가합니다. FR-START-002의 단계 순서와 FR-START-005의 버튼 조건이 바뀝니다.
  - 셸 선택의 보관을 추가합니다.
  - 모델 목록 조회(FR-START-007)가 같은 실행 환경을 쓰게 합니다.

## Impact

- `src-tauri/src/platform/macos.rs`: 로그인 셸에서 환경 전체를 읽어 감지(`AgentHost`), 모델 조회와 로그인 확인(`AgentCli`), 훅 설치 대상에 넘깁니다.
- `src-tauri/src/platform/windows.rs`:
  - 고른 셸(PowerShell, Git Bash)로 환경을 읽습니다.
  - Git Bash 설치 여부를 확인합니다.
  - WSL 실행 명령을 로그인 셸 방식으로 바꿉니다.
  - 훅 설치 대상 폴더를 바꿉니다.
- `src-tauri/src/agent/domain/`: 셸 출력에서 환경을 꺼내는 순수 함수, 셸 선택 타입.
- `src-tauri/src/hook/`, `src-tauri/src/model/`, `src-tauri/src/login/`: 설정 폴더 경로를 홈 기준 고정값 대신 실행 환경에서 계산합니다.
- `src-tauri/src/commands/`, `state.rs`: 셸 선택을 읽고 저장하는 명령과 보관.
- `src/renderer/src/features/onboarding/` 등: Windows에서만 보이는 셸 선택 단계와 스테퍼 단계 수 변경. 인증 값은 IPC로 오가지 않습니다.
- 의존성 추가 없음.
- 정할 것(clarify):
  - 셸 선택 단계의 위치(감지 앞인지 뒤인지)
  - 첫 실행 때 시작 감지에 쓸 환경
  - 선택을 바꾼 뒤 다시 감지할지
  - PowerShell 7(`pwsh`)과 Windows PowerShell 중 무엇을 쓸지
  - 셸이 느리거나 실패할 때의 시간 상한과 대체 동작
