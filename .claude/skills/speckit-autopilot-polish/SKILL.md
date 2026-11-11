---
name: speckit-autopilot-polish
description: 'autopilot: 사람이 검토하기 직전(GATE-1, GATE-2)에 spec과 plan의 한국어 문장을 humanizer로 다듬는다'
compatibility: Requires spec-kit project structure with .specify/ directory
metadata:
  author: namaste-world
  source: autopilot:commands/speckit.autopilot.polish.md
---

## 입력

```text
$ARGUMENTS
```

첫 단어가 모드입니다: `gate1`, `gate2`. 현재 기능 디렉터리는 `.specify/feature.json`의 `feature_directory`입니다(아래에서 `<기능>`).

| 모드 | 다듬을 파일 |
| --- | --- |
| `gate1` | `<기능>/spec.md` |
| `gate2` | `<기능>/spec.md`, `<기능>/plan.md` |

## 하는 일

루프의 다른 단계는 `humanizer`를 부르지 않습니다. 사람이 문서를 읽기 직전인 이 단계에서만 한 번 다듬습니다.

1. `.agents/skills/humanizer/SKILL.md`를 읽고 그 절차로 위 파일의 한국어 문장을 다듬습니다.
2. 문장 표현만 바꿉니다. 아래는 그대로 둡니다.
   - 뜻, 숫자, 단위, 조건, 요구사항의 범위
   - 섹션 제목, 필드 이름(`**Domain**:` 등), ID(`FR-XXX-NNN`, `US1`, `SC-001`), 코드와 경로
   - 표와 목록의 구조, `[NEEDS CLARIFICATION: ...]`, `[Source: ...]` 표시
3. 사용자에게 질문하지 않습니다. 끝나면 고친 파일과 고친 문장 수를 짧게 보고합니다.