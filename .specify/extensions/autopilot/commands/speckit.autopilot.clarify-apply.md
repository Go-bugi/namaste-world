---
description: "autopilot A3: 사람이 clarify 파일에 적은 답을 spec에 반영한다"
---

## 입력

```text
$ARGUMENTS
```

## 하는 일

1. 현재 기능 디렉터리(`.specify/feature.json`의 `feature_directory`)에서 가장 최근 `clarify/round-N.md`를 읽습니다.
2. `**답**:`이 채워진 질문마다 core clarify와 같은 방식으로 spec.md에 반영합니다.
   - `## Clarifications` 아래 `### Session YYYY-MM-DD`에 `- Q: <질문> → A: <답>`을 추가합니다.
   - 답이 영향을 주는 절(Functional Requirements, Acceptance Scenarios, Edge Cases, Key Entities, Success Criteria)을 고칩니다. 해당 `[NEEDS CLARIFICATION]` 표시는 지웁니다.
   - 답과 어긋나는 기존 문장은 지우거나 고칩니다. 같은 내용을 두 번 쓰지 않습니다.
3. round 파일 맨 끝에 `<!-- autopilot: resolved=K -->`를 붙입니다. K는 이번에 반영한 답의 수입니다.

## 하지 않는 일

- 답이 비어 있는 질문을 추측으로 채우지 않습니다.
- 답에 없는 새 요구사항을 만들지 않습니다.
