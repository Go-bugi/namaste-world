# Feature Specification: [FEATURE NAME]

**Feature Branch**: `[###-feature-name]`

**Created**: [DATE]

**Status**: Draft

**Domain**: [constitution "Spec Conventions"의 도메인 코드 하나. 표에 없으면 새 코드를 제안하고 GATE-1에서 정합니다]

**Touches**: [이 기능이 기대거나 바꾸는 기존 요구사항 ID. `.specify/memory/spec.md`와 `specs/<기능>/context.md`에서 찾습니다. 없으면 "없음"]

**Input**: User description: "$ARGUMENTS"

<!--
  작성 규칙 (namaste preset)
  - 본문은 한국어로 씁니다. 섹션 제목, 필드 이름(Why this priority 등), Given/When/Then,
    ID 접두사는 spec-kit 명령과 archive가 찾을 수 있게 영어 그대로 둡니다.
  - 요구사항 ID는 FR-<도메인>-NNN, 성공 기준은 SC-<도메인>-NNN입니다.
  - 기존 요구사항을 바꾸거나 없애면 그 ID를 Touches에 적고, 해당 FR 본문에 "FR-SIG-003을 대체"처럼 밝힙니다.
  - 이 spec만 보고 테스트를 쓸 수 있어야 합니다. 모든 Acceptance Scenario와 FR은 관찰할 수 있는 결과
    (값, 화면에 보이는 것, 저장된 것, 에러 종류)로 끝나야 하고, "빠르게", "자연스럽게" 같은 말에는 숫자 기준을 붙입니다.
  - 구현 방법(파일, 함수, 라이브러리)은 쓰지 않습니다. plan의 몫입니다.
-->

## User Scenarios & Testing *(mandatory)*

<!--
  사용자 스토리는 중요도 순으로 씁니다. 각 스토리는 그것만 구현해도 쓸 수 있는 조각이어야 합니다.
-->

### User Story 1 - [Brief Title] (Priority: P1)

[사용자가 겪는 흐름을 쉬운 말로]

**Why this priority**: [이 우선순위인 이유]

**Independent Test**: [이 스토리만 따로 확인하는 방법]

**Acceptance Scenarios**:

1. **Given** [초기 상태], **When** [행동], **Then** [관찰할 수 있는 결과]
2. **Given** [초기 상태], **When** [행동], **Then** [관찰할 수 있는 결과]

---

### User Story 2 - [Brief Title] (Priority: P2)

[사용자가 겪는 흐름]

**Why this priority**: [이유]

**Independent Test**: [확인 방법]

**Acceptance Scenarios**:

1. **Given** [초기 상태], **When** [행동], **Then** [관찰할 수 있는 결과]

---

[필요한 만큼 스토리를 추가합니다]

### Edge Cases

- [경계 조건]에서는 어떻게 되는가?
- [에러 상황]을 어떻게 다루는가? 사용자에게 무엇이 보이는가?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-[DOMAIN]-001**: System MUST [구체적인 기능]
- **FR-[DOMAIN]-002**: System MUST [구체적인 기능]

*정하지 않은 항목 표시 예:*

- **FR-[DOMAIN]-003**: System MUST [기능] [NEEDS CLARIFICATION: 무엇이 정해지지 않았는지]

### Key Entities *(include if feature involves data)*

- **[Entity 1]**: [무엇을 나타내는지, 주요 속성. 구현 타입은 쓰지 않음]

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-[DOMAIN]-001**: [측정할 수 있는 기준]

## Assumptions

- [기능 설명에 없어서 합리적인 기본값으로 정한 것]
