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

macOS(로컬)에서 돌린 결과입니다. Rust 테스트 68개, TS 테스트 19개입니다.

## 요구사항 연결

모두 연결됨. 모든 FR-/SC- ID가 plan.md 작업에 나옵니다. FR-AGENT-006(감지 중에도 앱이 멈추지 않음)은 테스트 제목에는 없고, 작업 5.3에서 사람이 직접 확인합니다.

## 확인하지 못한 것

명령 검사에는 포함되지 않아 사람이 확인합니다.

- Windows 빌드와 WSL 감지(작업 3.2). macOS에서 Windows 타깃 교차 검사가 `tauri-build` 때문에 돌지 않았습니다.
- Linux CI의 `lint:rust`. 로컬에서 Linux 타깃 검사가 GTK `pkg-config` 때문에 돌지 않았습니다.
- Finder로 띄운 macOS 빌드의 감지(작업 3.1), `pnpm dev` 화면 동작(작업 5.3, FR-AGENT-006).

결과: 통과
plan-hash: ad1726123bd48bb2
