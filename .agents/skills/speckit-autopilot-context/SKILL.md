---
name: speckit-autopilot-context
description: 'autopilot A1: 기존 memory에서 이번 기능과 관련된 요구사항과 결정만 골라 context.md에 쓴다'
compatibility: Requires spec-kit project structure with .specify/ directory
metadata:
  author: namaste-world
  source: autopilot:commands/speckit.autopilot.context.md
---

## 입력

```text
$ARGUMENTS
```

입력은 이번 기능 설명입니다. 파일 경로 하나라면 그 파일을 읽어 기능 설명으로 씁니다.

## 하는 일

새 기능의 spec을 쓰기 전에, 이미 구현된 기능 가운데 이번 기능과 관련된 것만 골라 정리합니다. 이 결과는 specify와 충돌 검사(A5)가 씁니다.

1. 다음을 읽습니다.
   - `.specify/memory/constitution.md`
   - `.specify/memory/spec.md`
   - `.specify/memory/plan.md`
   - `.specify/memory/changelog.md`의 `## Unresolved Contradictions`
2. 기능 설명과 같은 사용자 목표, 같은 엔티티, 같은 화면이나 명령, 같은 데이터를 다루는 항목을 고릅니다. 애매하면 넣습니다. 빠뜨리는 쪽이 더 위험합니다.
3. `.specify/autopilot/context.md`에 아래 형식으로 씁니다. 디렉터리가 없으면 만듭니다.

```markdown
# Context: <기능 이름>

## 관련 기존 요구사항
- FR-XXX-NNN: <한 줄 요약> [Source: ...]

## 관련 엔티티
- <엔티티>: <한 줄 요약>

## 관련 기술 결정
- <plan.md 항목 한 줄>

## 관련 constitution 원칙
- <원칙 번호와 이름>

## 미해결 모순
- <changelog의 항목 중 관련된 것>
```

memory에 기능 내용이 아직 없으면(섹션 제목만 있으면) "관련 기존 요구사항" 아래에 `- 기존 기능 없음`이라고만 쓰고 나머지 절은 constitution 항목만 채웁니다.

## 하지 않는 일

- `.specify/autopilot/context.md` 말고 다른 파일을 만들거나 고치지 않습니다.
- 사용자에게 질문하지 않습니다. 이 명령은 사람 없이 실행됩니다.