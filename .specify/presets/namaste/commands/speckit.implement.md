## namaste 저장소 규칙 (preset 보강)

위 절차에 아래를 더합니다. 위 절차와 어긋나면 아래를 따릅니다.

0. **autopilot 실행**: 인자가 `[autopilot]`으로 시작하면 autopilot 루프의 한 단계로 실행된 것입니다. 사용자에게 묻지 않고 끝까지 진행합니다.
   - 체크리스트 확인(위 절차의 checklists 상태 점검)에서 체크되지 않은 항목이 있어도 묻지 않고 진행합니다. 체크리스트는 GATE-1에서 사람이 이미 보았습니다. 상태 표는 출력에 남깁니다.
   - 인자의 나머지 문장이 처리할 범위(Phase)를 정합니다.
1. **code-writer 절차**: 작업마다 `.agents/skills/code-writer/SKILL.md`의 절차를 따릅니다. 작업의 `[Layer]` 표시로 라우팅 표에서 부를 스킬을 골라 그 SKILL.md를 먼저 읽습니다.
   - Phase 1(Modeling)은 2단계, Phase 3(Core)은 4단계, Phase 4(Shell)는 5단계, Phase 5(Verification)는 6단계입니다.
   - Phase 2(Tests)의 테스트가 이미 있으면 그 작업은 체크만 하고 넘어갑니다.
2. **테스트 파일은 고치지 않습니다**: `*.test.ts`, `*.test.tsx` 파일과 Rust 파일의 `#[cfg(test)] mod tests` 부분을 수정하거나 지우지 않습니다. 테스트가 틀렸다고 판단하면 고치지 말고 `specs/<기능>/test-disputes.md`에 테스트 이름, 근거가 되는 spec 문장, 이유를 적고 그 작업을 남겨 둡니다.
3. **spec 밖으로 나가지 않습니다**: spec과 contracts에 없는 동작을 추가하지 않습니다.
4. **완료 기준**: 모든 작업이 끝나면 `code-writer` "완료 기준"의 검사를 돌리고 결과를 보고합니다. 실패한 검사는 출력과 함께 보고합니다.
