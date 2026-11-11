# Implementation Plan: [FEATURE]

**Branch**: `[###-feature-name]` | **Date**: [DATE] | **Spec**: [link]

**Input**: Feature specification from `/specs/[###-feature-name]/spec.md`

<!--
  작성 규칙 (namaste preset)
  - 이 plan은 code-writer 스킬의 0~2단계(맥락, 층 분류, 모델링) 결과를 적는 문서입니다.
    먼저 `.agents/skills/code-writer/SKILL.md`, `.agents/skills/fp-apply/SKILL.md`, `AGENTS.md`의
    "구조"와 "폴더 구조"를 읽습니다. 코드 규칙은 여기에 옮겨 적지 않고 그 문서를 따릅니다.
  - 본문은 한국어로 씁니다. 섹션 제목은 영어 그대로 둡니다(archive가 이 제목으로 합칩니다).
-->

## Summary

[spec의 핵심 요구사항과 기술 방향 요약]

## Technical Context

**Language/Version**: TypeScript(renderer), Rust stable(src-tauri)

**Primary Dependencies**: Tauri 2, React, [이번 기능에서 쓰는 라이브러리]

**Storage**: [해당하면]

**Testing**: Vitest + fast-check(`pnpm test`), `cargo test`(`pnpm test:rust`)

**Target Platform**: macOS(WKWebView, safari13 타깃), Windows(WebView2)

**Project Type**: desktop-app

**Performance Goals**: [해당하면 숫자로]

**Constraints**: [해당하면]

**Scale/Scope**: [해당하면]

## Constitution Check

*GATE: Phase 0 전에 통과하고, Phase 1 설계 뒤에 다시 확인합니다.*

- [ ] I. 코드 규칙: 아래 "Layers & Skills"와 "Core Model"이 `code-writer`, `fp-apply`, `AGENTS.md`를 따른다
- [ ] II. 테스트 먼저: 테스트 대상 공개 함수가 `contracts/`에 시그니처로 있다
- [ ] III. 기존 스펙: spec의 Touches에 적힌 ID와 충돌하지 않거나, 대체를 spec에 밝혔다
- [ ] IV. 범위: spec에 없는 동작을 넣지 않았다

## Layers & Skills

<!-- code-writer 1단계. 이번 기능이 건드리는 층마다 한 줄. 부를 스킬은 code-writer 라우팅 표에서 고릅니다. -->

| 층 | 위치(`AGENTS.md` "폴더 구조" 기준) | 부를 스킬 | 하는 일 |
| --- | --- | --- | --- |
| core | [예: `src/renderer/src/features/<기능>/domain/`] | `fp-apply` | [계산] |
| IO | [예: `src-tauri/src/commands/`, `src/renderer/src/api/`] | `fp-apply` `references/rust.md` | [명령, 이벤트] |
| [상태, React, 3D, 설정 중 해당하는 것] | | | |

## Core Model

<!--
  code-writer 2단계. 타입, 상태 전이, 에러 유니온, 공개 함수 시그니처를 정합니다.
  테스트 대상 시그니처는 contracts/에 그대로 옮깁니다. test-writer 서브에이전트는 plan을 보지 않고
  spec과 contracts만 보고 테스트를 씁니다. 그래서 contracts에는 시그니처, 입력과 출력의 의미,
  에러 경우를 빠짐없이 적습니다.
-->

- **Types**: [도메인 타입, 판별 유니온]
- **Functions**: [시그니처와 한 줄 설명. 예: `nextSalary(xp: Xp): Salary`]
- **Errors**: [에러 유니온과 각각이 나는 조건]

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md
├── research.md
├── data-model.md
├── contracts/           # 테스트 대상 공개 함수와 명령의 시그니처
├── tasks.md
├── context.md           # autopilot A1: 관련 기존 요구사항
├── conflicts.md         # autopilot A5: 기존 스펙과의 모순
└── loop/                # autopilot 검사 결과
```

### Source Code (repository root)

```text
[이번 기능이 만들거나 고치는 실제 경로만. AGENTS.md "폴더 구조"를 따름]
```

**Structure Decision**: [위 경로를 고른 이유]

## Complexity Tracking

> Constitution Check에 어긋나는 항목이 있을 때만 씁니다.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
