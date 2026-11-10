---
description: "autopilot B4-2: test-writer 서브에이전트를 띄워 spec과 contract만 보고 테스트를 쓰게 한다"
---

## 입력

```text
$ARGUMENTS
```

## 하는 일

테스트는 구현과 앞 단계 맥락을 모르는 서브에이전트가 씁니다. 이 명령을 실행하는 에이전트는 서브에이전트를 띄우고 결과만 전달합니다.

1. 현재 기능 디렉터리는 `.specify/feature.json`의 `feature_directory`입니다(아래에서 `<기능>`). 입력 묶음은 `<기능>/loop/test-input/`에 있습니다. **이 묶음과 spec, plan, 코드를 직접 읽지 않습니다.** 읽으면 그 내용이 서브에이전트에게 넘어가기 쉽습니다.
2. `test-writer` 서브에이전트를 띄웁니다.
   - Claude Code: Agent(Task) 도구로 `subagent_type: "test-writer"`를 부릅니다.
   - Codex: `test-writer` 에이전트를 하나 띄워(spawn) 끝날 때까지 기다립니다.
3. 서브에이전트에게 넘기는 내용은 아래뿐입니다.

```text
입력 묶음: <기능>/loop/test-input/
질문 파일: <기능>/test-questions.md
이의 파일: <기능>/test-disputes.md (있으면 이번 판정에 반영)
```

4. 서브에이전트가 끝나면 그가 보고한 테스트 파일 목록과 `test-questions.md` 질문 수만 출력합니다.

## 하지 않는 일

- 테스트를 직접 쓰거나 고치지 않습니다.
- 서브에이전트에게 spec 요약, 구현 힌트, 앞 단계 대화를 넘기지 않습니다.
