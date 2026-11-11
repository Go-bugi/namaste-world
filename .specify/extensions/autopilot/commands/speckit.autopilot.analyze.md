---
description: "autopilot B3, C1: core analyze와 같은 검사를 하고 결과를 loop/analyze.md에 쓴다"
---

## 입력

```text
$ARGUMENTS
```

## 하는 일

core analyze는 보고서를 화면에만 냅니다. autopilot은 결과를 파일로 판정해야 해서 이 명령이 같은 검사를 하고 파일에 씁니다.

1. `speckit-analyze` 스킬 파일(`.claude/skills/speckit-analyze/SKILL.md` 또는 `.agents/skills/speckit-analyze/SKILL.md`)을 읽고 그 절차대로 검사합니다. 심각도 기준(CRITICAL, HIGH, MEDIUM, LOW)과 최대 50개 제한도 그대로 따릅니다.
2. 그 절차가 "파일을 고치지 않는다"고 한 규칙은 지킵니다. 단 하나의 예외로, 보고서를 현재 기능 디렉터리(`.specify/feature.json`의 `feature_directory`)의 `loop/analyze.md`에 씁니다(이전 내용은 지웁니다).
3. 보고서 맨 마지막 줄은 정확히 이 형식입니다.

```text
COUNTS: critical=<수> high=<수> medium=<수> low=<수>
```

## 하지 않는 일

- spec, plan, tasks, 코드를 고치지 않습니다. 고치는 일은 `speckit.autopilot.fix analyze`가 합니다.
- 사용자에게 질문하거나 다음 명령을 제안하며 기다리지 않습니다.
