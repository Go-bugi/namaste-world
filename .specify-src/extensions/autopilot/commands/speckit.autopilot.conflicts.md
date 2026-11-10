---
description: "autopilot A5: 새 spec을 memory, constitution과 비교해 모순과 대체 후보를 conflicts.md에 쓴다"
---

## 입력

```text
$ARGUMENTS
```

## 하는 일

이 명령은 spec을 만든 에이전트와 다른 에이전트가 실행합니다. 새 spec이 이미 구현된 기능이나 프로젝트 원칙과 어긋나는지 봅니다.

1. 현재 기능 디렉터리(`.specify/feature.json`의 `feature_directory`)의 `spec.md`, `context.md`(있으면)를 읽습니다.
2. `.specify/memory/constitution.md`, `.specify/memory/spec.md`, `.specify/memory/plan.md`, `.specify/memory/changelog.md`의 `## Unresolved Contradictions`를 읽습니다.
3. 비교 기준은 archive 확장의 모순 판정과 같습니다. 같은 대상(엔티티, 화면, 명령, 데이터)에 대해 두 문장이 동시에 참일 수 없으면 모순입니다. 범위가 다르거나 서로 보완하면 모순이 아닙니다.
4. 현재 기능 디렉터리의 `conflicts.md`를 새로 씁니다(이전 내용은 지웁니다).

```markdown
# Conflicts

## 기존 요구사항과의 관계
- [모순] FR-NEW-002 vs FR-OLD-005 (memory): <어떻게 어긋나는지 한 줄>
- [대체] FR-NEW-003 → FR-OLD-001 (memory): spec이 대체를 밝혔음
- [모순] FR-NEW-004 vs constitution II: <어긋나는 이유>

## 확인한 범위
- <읽은 파일과, 비교한 memory 항목 수>
```

- spec 본문이 "FR-XXX를 대체"처럼 대체를 밝힌 경우만 `[대체]`로 씁니다. 밝히지 않았으면 `[모순]`입니다.
- constitution 위반도 `- [모순] ... vs constitution <원칙>` 형식으로 씁니다.
- 모순이 없으면 "기존 요구사항과의 관계" 아래에 `- 없음`이라고 씁니다.

## 하지 않는 일

- `conflicts.md` 말고 다른 파일을 고치지 않습니다.
