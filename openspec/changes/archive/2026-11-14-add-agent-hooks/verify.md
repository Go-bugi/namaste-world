# Verify

| 명령                     | 결과 |
| ------------------------ | ---- |
| `pnpm format:check:rust` | 통과 |
| `pnpm lint:rust`         | 통과 |
| `pnpm test:rust`         | 통과 |
| `pnpm format:check`      | 통과 |
| `pnpm lint`              | 통과 |
| `pnpm test`              | 통과 |
| `pnpm build`             | 통과 |

macOS(로컬)에서 돌린 결과입니다. Rust 테스트 197개, TS 테스트 35개입니다. Windows 타깃(`x86_64-pc-windows-msvc`) clippy도 `llvm-rc` 자리에 가짜 실행 파일을 두고 경고 없이 통과했습니다.

## 요구사항 연결

모두 연결됨. 모든 FR-/SC- ID가 plan.md 작업에 나옵니다. 아래 넷은 테스트 제목에는 없고 작업의 실제 환경 확인으로 검증했습니다.

- FR-HOOK-005(에이전트를 방해하지 않는 훅): 작업 4.1, 4.2, 9.1, 9.2. 훅 스크립트가 출력 없이 0으로 끝나고, 앱이 꺼져 있어도 에이전트가 평소처럼 응답함
- FR-HOOK-012(이벤트 내용을 디스크에 남기지 않음): 작업 7.2. 서버와 작업이 이벤트 내용을 로그로 남기지 않음
- FR-HOOK-014(앱은 하나만 실행): 작업 7.2, 9.1. 두 번째 실행이 바로 끝나고 켜진 창이 앞으로 나옴
- SC-HOOK-002(이벤트 표시 지연): 작업 8.3, 9.1. Claude Code와 Codex 이벤트가 질문 직후 화면에 나옴

## 확인하지 못한 것

- Linux CI의 `lint:rust`. 로컬에서 Linux 타깃 검사는 돌리지 않았습니다.
- Windows와 WSL 동작(작업 4.2, 9.2)은 사용자가 Windows에서 확인했다고 알려 준 결과입니다.

결과: 통과
plan-hash: 4ca0bc67b4844fc1
