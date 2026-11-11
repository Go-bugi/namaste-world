---

description: "Task list template for feature implementation (namaste preset)"
---

# Tasks: [FEATURE NAME]

**Input**: Design documents from `/specs/[###-feature-name]/`

**Prerequisites**: plan.md (required), spec.md (required), contracts/ (required), research.md, data-model.md

**Tests**: 테스트는 항상 씁니다(constitution II). autopilot 루프에서는 Phase 2의 테스트 작업을 `test-writer` 서브에이전트가 맡고, 구현 작업을 하는 에이전트는 테스트 파일을 고치지 않습니다.

**Organization**: 작업 순서는 `code-writer` 스킬의 단계(모델링, 테스트, core 구현, shell 연결, 검증)를 따릅니다. 각 Phase 안에서는 사용자 스토리 순서로 나눕니다.

## Format: `[ID] [P?] [Layer] [Story] Description`

- **[P]**: 다른 파일을 건드려 동시에 할 수 있는 작업
- **[Layer]**: `core`, `state`, `react`, `3d`, `io`, `config` 중 하나. `code-writer` 라우팅 표에서 이 층의 스킬을 고릅니다
- **[Story]**: 해당 사용자 스토리(US1, US2 등)
- 설명에는 실제 파일 경로를 적습니다(`AGENTS.md` "폴더 구조" 기준)

<!--
  아래 작업은 예시입니다. /speckit-tasks는 spec, plan, contracts를 보고 실제 작업으로 바꿉니다.
  Phase 순서는 바꾸지 않습니다. 순서가 code-writer 단계와 같아야 autopilot 루프가 단계별로 실행할 수 있습니다.
-->

## Phase 1: Modeling (code-writer 2단계)

**Purpose**: `contracts/`의 타입과 함수 시그니처를 본문 없이 만듭니다. TS는 본문을 `throw new Error('not implemented')`, Rust는 `todo!()`로 둡니다. typecheck가 통과해야 합니다.

- [ ] T001 [core] [US1] `src/renderer/src/features/[기능]/domain/[파일].ts`에 타입과 시그니처 작성

---

## Phase 2: Tests (code-writer 3단계)

**Purpose**: spec의 Acceptance Scenario와 FR마다 테스트를 씁니다. 테스트는 이 시점에 실패해야 합니다(red).

- [ ] T002 [P] [core] [US1] `[파일].test.ts`에 FR-[DOMAIN]-001 테스트

---

## Phase 3: Core Implementation (code-writer 4단계)

**Purpose**: Phase 1의 시그니처에 본문을 채워 테스트를 통과시킵니다.

- [ ] T003 [core] [US1] `[파일].ts` 구현

---

## Phase 4: Shell Wiring (code-writer 5단계)

**Purpose**: store, 컴포넌트, `useFrame`, `api/`, Rust 명령을 연결합니다. shell은 core를 부르기만 합니다.

- [ ] T004 [io] [US1] `src-tauri/src/commands/[파일].rs`에 명령 추가
- [ ] T005 [react] [US1] `src/renderer/src/features/[기능]/[컴포넌트].tsx` 연결

---

## Phase 5: Verification (code-writer 6단계)

**Purpose**: `AGENTS.md` "명령어"의 PR CI 검사를 로컬에서 돌립니다. Rust를 건드렸으면 Rust 검사를 먼저 돌립니다.

- [ ] T006 `pnpm test:rust`, `pnpm lint:rust`, `pnpm format:check:rust` 통과
- [ ] T007 `pnpm test`, `pnpm lint`, `pnpm format:check`, `pnpm build` 통과

---

## Dependencies & Execution Order

- Phase 1 → 2 → 3 → 4 → 5 순서로 진행합니다. 앞 Phase가 끝나야 다음으로 갑니다.
- 같은 Phase 안의 [P] 작업은 동시에 할 수 있습니다.
