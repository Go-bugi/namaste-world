## namaste 저장소 규칙 (preset 보강)

위 절차에 아래를 더합니다. 위 절차와 어긋나면 아래를 따릅니다.

0. **autopilot 실행**: 인자가 `[autopilot]`으로 시작하면 autopilot 루프의 한 단계로 실행된 것입니다. 사용자에게 묻지 않고 끝까지 진행합니다.
   - 체크리스트 확인(위 절차의 checklists 상태 점검)에서 체크되지 않은 항목이 있어도 묻지 않고 진행합니다. 체크리스트는 GATE-1에서 사람이 이미 보았습니다. 상태 표는 출력에 남깁니다.
   - 인자의 나머지 문장이 처리할 범위(Phase)를 정합니다.
   - **Phase 1(Modeling)만 하라는 인자면** 시그니처만 만드는 가벼운 단계로 실행합니다. 아래가 위 절차와 1, 4번 규칙보다 앞섭니다.
     - 읽는 파일: `tasks.md`의 Phase 1, `contracts/`, 시그니처를 넣을 기존 소스 파일, `AGENTS.md`의 "폴더 구조"만 읽습니다. memory 파일, `plan.md`, `research.md`, `data-model.md`, `quickstart.md`, 스킬 문서, `.specify/`와 `.specify-src/`의 스크립트는 읽지 않습니다. before 훅(memory-loader)도 실행하지 않습니다.
     - 검사: 모든 Phase 1 작업을 마친 뒤 `pnpm typecheck`와 `cargo check --tests --manifest-path src-tauri/Cargo.toml`을 한 번씩만 돌리고, 실패하면 고칩니다. lint, format 등 완료 기준 검사는 돌리지 않습니다. 루프의 검사 단계가 따로 확인합니다.
1. **code-writer 절차**: 작업마다 `.agents/skills/code-writer/SKILL.md`의 절차를 따릅니다. 작업의 `[Layer]` 표시로 라우팅 표에서 부를 스킬을 골라 그 SKILL.md를 먼저 읽습니다.
   - Phase 1(Modeling)은 2단계, Phase 3(Core)은 4단계, Phase 4(Shell)는 5단계, Phase 5(Verification)는 6단계입니다.
   - Phase 2(Tests)의 테스트가 이미 있으면 그 작업은 체크만 하고 넘어갑니다.
2. **테스트 파일은 고치지 않습니다**: `*.test.ts`, `*.test.tsx` 파일과 Rust 파일의 `#[cfg(test)] mod tests` 부분을 수정하거나 지우지 않습니다. 테스트가 틀렸다고 판단하면 고치지 말고 `specs/<기능>/test-disputes.md`에 테스트 이름, 근거가 되는 spec 문장, 이유를 적고 그 작업을 남겨 둡니다.
3. **spec 밖으로 나가지 않습니다**: spec과 contracts에 없는 동작을 추가하지 않습니다.
4. **완료 기준**: 모든 작업이 끝나면 `code-writer` "완료 기준"의 검사를 돌리고 결과를 보고합니다. 실패한 검사는 출력과 함께 보고합니다.
