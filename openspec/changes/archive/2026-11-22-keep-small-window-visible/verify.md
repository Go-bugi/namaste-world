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

## 요구사항 연결

모두 연결됨. FR-WINDOW-006, FR-WINDOW-010은 plan 작업과 `placement.rs` 테스트에, FR-WINDOW-001, FR-WINDOW-009, SC-WINDOW-002는 plan 작업(수동 확인)에 나온다.

수동 확인 중 작업 3.2(다시 불러낼 때 위치, FR-WINDOW-006), 4.2(모니터 변경 시 옮기기, FR-WINDOW-010, SC-WINDOW-002), 5.1(Windows 항상 위, FR-WINDOW-001)은 사용자가 나중에 확인하기로 해 아직 실제 앱에서 확인하지 않았다.

결과: 통과
plan-hash: 26c42c220de0892d
