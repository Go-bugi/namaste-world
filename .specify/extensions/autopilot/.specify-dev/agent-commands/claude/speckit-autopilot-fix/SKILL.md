---
name: speckit-autopilot-fix
description: 'autopilot: 검사 결과를 보고 spec, plan, tasks 또는 코드를 고친다(checklist, conflicts, gate1, analyze, verify)'
compatibility: Requires spec-kit project structure with .specify/ directory
metadata:
  author: namaste-world
  source: autopilot:commands/speckit.autopilot.fix.md
---

## 입력

```text
$ARGUMENTS
```

첫 단어가 모드입니다: `checklist`, `conflicts`, `gate1`, `analyze`, `verify`. 현재 기능 디렉터리는 `.specify/feature.json`의 `feature_directory`입니다(아래에서 `<기능>`).

## 공통 규칙

- 검사 결과에 적힌 문제만 고칩니다. 범위를 넓히지 않습니다.
- 새 요구사항, 새 값, 새 동작을 지어내지 않습니다. 고치려면 사람이 정해야 하는 문제는 고치지 말고 아래 모드별 방법으로 "결정 필요"를 남깁니다.
- 사용자에게 질문하지 않습니다. 이 명령은 사람 없이 실행됩니다.
- 끝나면 무엇을 고쳤고 무엇을 남겼는지 짧게 보고합니다.

## 모드

### checklist

`<기능>/checklists/testability.md`에서 체크되지 않은(`- [ ]`) 항목 중 `[Gap]`, `[Ambiguity]`, `[Conflict]` 표시가 있고 `[결정 필요]`가 없는 항목을 봅니다.

- 문장만 고치면 되는 항목(측정 기준이 빠진 표현, 관찰할 수 없는 결과, 서로 다른 용어)은 `spec.md`를 고칩니다.
- 값이나 동작을 새로 정해야 하는 항목은 spec을 고치지 말고 checklist 항목 끝에 `[결정 필요]`를 붙입니다.
- checklist의 `[x]` 체크는 하지 않습니다. 다음 checklist 실행이 다시 판정합니다.

### conflicts

`<기능>/conflicts.md`의 `[모순]` 줄을 봅니다.

- 기능 설명(spec의 `**Input**`)이 기존 동작을 바꾸려는 뜻이 분명하면, 해당 FR 본문에 "FR-XXX를 대체"를 밝히고 `**Touches**`에 그 ID를 넣습니다.
- 그런 뜻이 없으면 spec을 기존 요구사항에 맞춥니다.
- constitution 위반은 spec을 원칙에 맞춥니다. 원칙 자체를 바꿔야 한다고 보면 고치지 말고 `conflicts.md` 그 줄 끝에 `[결정 필요]`를 붙입니다.
- `conflicts.md`는 다음 검사가 새로 씁니다. 직접 지우지 않습니다.

### gate1

사람이 GATE-1에서 거절하며 남긴 `<기능>/loop/gate1-feedback.md`를 읽고 `spec.md`를 고칩니다. 피드백에 없는 부분은 고치지 않습니다. 파일이 없거나 비어 있으면 아무것도 고치지 않고 그렇게 보고합니다.

### analyze

`<기능>/loop/analyze.md`의 CRITICAL과 HIGH 항목을 봅니다. `spec.md`, `plan.md`, `tasks.md`, `contracts/`를 고쳐 해소합니다. 코드는 아직 없거나 고치지 않습니다. 해소하려면 요구사항을 새로 정해야 하는 항목은 고치지 말고 보고에 남깁니다.

### verify

`<기능>/loop/verify-run.json`의 실패한 검사와 `<기능>/loop/verify/` 로그, `<기능>/loop/review.md`의 지적을 봅니다.

- `.agents/skills/code-writer/SKILL.md` 절차와 라우팅 표의 스킬을 따라 코드를 고칩니다.
- 테스트 파일(`*.test.ts`, `*.test.tsx`, Rust 파일의 `#[cfg(test)] mod tests`)은 고치지 않습니다. 테스트가 틀렸다고 보면 `<기능>/test-disputes.md`에 테스트 이름, 근거가 되는 spec 문장, 이유를 적습니다.
- 검사를 끄거나(`eslint-disable`, `#[allow]`, `@ts-ignore` 등) 설정을 바꿔 통과시키지 않습니다.
- 서식 오류는 `pnpm format`, `pnpm format:rust`로 고칩니다.