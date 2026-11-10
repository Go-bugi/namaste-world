---
description: "autopilot A3: clarify 질문을 최대 5개 골라 파일로 쓴다(사람이 파일에 답한다)"
---

## 입력

```text
$ARGUMENTS
```

## 하는 일

core clarify는 대화로 질문하고 답을 받습니다. autopilot은 사람 없이 돌기 때문에 질문을 파일에 쓰고, 사람이 그 파일에 답을 적습니다.

1. 현재 기능 디렉터리는 `.specify/feature.json`의 `feature_directory`입니다. 그 안의 `spec.md`와 `context.md`(있으면)를 읽습니다.
2. 질문을 고르는 기준은 core clarify와 같습니다. `speckit-clarify` 스킬 파일(`.claude/skills/speckit-clarify/SKILL.md` 또는 `.agents/skills/speckit-clarify/SKILL.md`)의 범주(taxonomy)와 우선순위 기준으로, 답에 따라 설계나 테스트가 달라지는 질문만 최대 5개 고릅니다. 이미 `## Clarifications`에 답이 있는 질문은 다시 묻지 않습니다.
3. `clarify/` 안의 기존 `round-*.md` 수에 1을 더한 N으로 `clarify/round-N.md`를 씁니다.

```markdown
# Clarify round N

질문마다 **답**: 뒤에 답을 적습니다. 선택지 기호만 적어도 됩니다. 비워 둔 질문은 이번 라운드에서 넘어갑니다.

### Q1: <질문>

- **왜 묻나**: <답에 따라 무엇이 달라지는지 한 줄>
- **선택지**: A) ... B) ... C) ...
- **추천**: <선택지와 이유 한 줄>
- **답**:
```

물을 것이 없으면 `# Clarify round N` 아래에 `질문 없음`과 `<!-- autopilot: resolved=0 -->`만 씁니다.

## 하지 않는 일

- spec.md를 고치지 않습니다. 반영은 `speckit.autopilot.clarify-apply`가 합니다.
- 사용자에게 대화로 질문하지 않습니다.
