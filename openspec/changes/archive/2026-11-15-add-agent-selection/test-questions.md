# 테스트 질문

### TQ1. Claude 초기화 응답의 바깥 모양이 `{"type":"control_response","response":{"subtype":"success","request_id":"<id>","response":{"models":[...]}}}`가 맞나요?

- 근거: FR-START-007, plan.md 설계 "시그니처" `claude_models`, "위험"의 "해석 테스트는 이번에 받은 실제 응답을 바탕으로 합니다"
- `parse.rs`와 `query.rs` 테스트는 이 모양으로 썼습니다. 설계에는 모델 항목의 필드(value, displayName, description)만 있고 바깥 모양은 없습니다. 실제로 받은 응답과 다르면 그 응답 한 줄을 알려 주세요.

**답**: 맞음. 사람이 다시 확인을 요청해 이 Mac의 Claude Code 2.1.289와 2.1.325에 같은 요청을 보내 확인함. 둘 다 `{"type":"control_response","response":{"subtype":"success","request_id":"<보낸 id>","response":{"models":[...], ...}, ...}}`. 지원하지 않는 요청이면 `subtype: "error"`와 `error` 필드가 오며 이때는 None

### TQ2. `savedLoaded`가 `detected`보다 늦게 오면 저장한 선택을 그때 미리 골라야 하나요?

- 근거: FR-START-006, plan.md 설계 `selectionReducer`의 detected 설명
- 설계에는 detected가 saved를 보고 고른다고만 있습니다. 테스트는 savedLoaded → detected 순서만 확인합니다. 반대 순서일 때 고르는지, 이미 사용자가 골랐으면 덮어쓰지 않는지 정해 주세요.

**답**: 역순은 없음. 앱은 저장한 선택을 먼저 읽고(`savedLoaded`) 그 뒤에 감지 결과(`detected`)를 받는다. 역순은 테스트하지 않음. 저장값이 사용자가 직접 고른 값을 덮는 일은 없음

### TQ3. `corner_rect`에서 창이 한 축만 작업 영역보다 크거나, 창은 들어가지만 여백까지는 들어가지 않으면 어떻게 하나요?

- 근거: FR-WINDOW-005, plan.md 설계 `corner_rect` "창이 작업 영역보다 크면 작업 영역 크기로 줄이고(여백 0)"
- 테스트는 두 축 모두 클 때(작업 영역과 같은 사각형)와 "결과는 항상 작업 영역 안" 속성만 확인합니다. 축마다 따로 줄이는지, 여백을 줄여 붙이는지 정해 주세요.

**답**: 축마다 따로. 넘치는 축만 작업 영역 크기로 줄이고, 여백은 그 축에 남는 만큼만(0~16×배율) 둔다

### TQ4. 배율을 곱한 크기와 여백이 정수가 아니면 어떻게 반올림하나요?

- 근거: FR-WINDOW-004, plan.md 설계 `corner_rect`
- 테스트는 결과가 정수로 떨어지는 배율(100%, 125%, 150%, 175%, 200%, 300%)만 씁니다. 반올림, 내림, 올림 중 무엇인지 정해 주세요.

**답**: 반올림. 결과가 작업 영역을 넘으면 넘지 않게만 보정

### TQ5. `codex debug models`의 모델에서 `ModelOption.detail`에는 무엇을 넣나요?

- 근거: FR-START-004, plan.md 설계 `codex_models` "label은 display_name, 없으면 slug"
- value는 slug, label은 display_name으로 확인하고 detail은 확인하지 않습니다. description을 넣는지, 비워 두는지 정해 주세요.

**답**: 설명문은 두 에이전트 모두 넣지 않음. Claude 별칭(value와 실제 모델 ID가 다른 항목)은 detail에 실제 모델 ID(`resolvedModel`, 예: `claude-opus-5-5`)만 넣고, 나머지와 Codex는 detail 없음. 스펙 FR-START-004를 고쳐야 해서 앞 단계(specs)로 돌아감

### TQ6. 다시 감지 중(`detecting: true`, 이전 결과 있음)일 때 훅 설치 단계와 선택 단계는 어떤 상태인가요?

- 근거: FR-START-002, FR-HOOK-015 시나리오 2, plan.md 설계 `stepStatuses`
- 테스트는 이때 감지 단계가 running인 것만 확인합니다. 설치 단계와 선택 단계를 pending으로 되돌리는지, 이전 상태(done/running)로 두는지 정해 주세요.

**답**: 대기로 돌아감. 다시 감지 중에는 훅 설치와 선택 단계를 pending으로 둔다. 이전 설치 결과와 고른 값은 화면에 남고, 다음 버튼은 막힘
