## namaste 저장소 규칙 (preset 보강)

위 절차에 아래를 더합니다. 위 절차와 어긋나면 아래를 따릅니다.

1. **Phase 순서 고정**: 템플릿의 Phase 1 Modeling, Phase 2 Tests, Phase 3 Core Implementation, Phase 4 Shell Wiring, Phase 5 Verification 순서를 그대로 씁니다. 사용자 스토리는 각 Phase 안에서 나눕니다.
2. **테스트 작업은 항상 씁니다**: spec이 테스트를 요구하지 않아도 Phase 2를 만듭니다(constitution II). `contracts/`의 공개 함수마다, spec의 FR과 Acceptance Scenario마다 테스트 작업이 하나 이상 있어야 합니다.
3. **층 표시**: 모든 작업에 `[core]`, `[state]`, `[react]`, `[3d]`, `[io]`, `[config]` 중 하나를 붙입니다.
4. **경로**: 작업 설명에 `AGENTS.md` "폴더 구조"에 맞는 실제 파일 경로를 적습니다.
