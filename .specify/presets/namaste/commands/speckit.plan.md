## namaste 저장소 규칙 (preset 보강)

위 절차에 아래를 더합니다. 위 절차와 어긋나면 아래를 따릅니다.

1. **먼저 읽을 것**: `AGENTS.md`("구조", "폴더 구조", "스킬"), `.agents/skills/code-writer/SKILL.md`, `.agents/skills/fp-apply/SKILL.md`, 이번에 건드릴 층의 `fp-apply` 언어별 문서(`references/typescript.md`, `references/rust.md` 등). 코드 규칙을 plan에 옮겨 적지 않고 이 문서들을 따릅니다.
2. **code-writer 0~2단계를 plan에 적기**: "Layers & Skills" 표에 건드리는 층, 위치, 부를 스킬을 적고, "Core Model"에 타입, 상태 전이, 에러 유니온, 공개 함수 시그니처를 적습니다. core 위치는 `AGENTS.md` "폴더 구조"를 따릅니다.
3. **contracts/**: 테스트할 공개 함수와 Rust 명령마다 `contracts/<이름>.md`를 씁니다. 각 파일에는 다음을 적습니다.
   - 실제로 만들 파일 경로와 시그니처(TS 또는 Rust 문법 그대로)
   - 입력과 출력의 의미, 단위, 허용 범위
   - 에러 경우와 각 에러가 나는 조건
   - 관련 FR과 Acceptance Scenario ID

   test-writer 서브에이전트는 plan을 보지 않고 spec과 contracts만 보고 테스트를 씁니다. contracts에 빠진 동작은 테스트되지 않습니다.
4. **사용자 기술 방향**: 사용자 입력이 있으면 기술 방향으로 받아들입니다. 입력이 파일 경로 하나라면 그 파일을 읽습니다. 입력이 `[autopilot]`으로 시작하면 그 표시를 떼고 나머지만 기술 방향으로 봅니다. 나머지가 비어 있으면 spec과 `AGENTS.md`를 보고 정합니다.
5. **한국어**: 본문은 한국어로, 섹션 제목은 영어 그대로 씁니다.
