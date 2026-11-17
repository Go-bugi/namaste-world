# Test Questions

### TQ1. `claude auth status --json`에서 `loggedIn`이 true인데 `authMethod`가 없으면 `method`는 비워 두나(`None`), `Other`로 담나?

- 근거: FR-AGENT-009("CLI가 로그인 방식을 알려 주면 결과에 함께 담는다"), `plan.md` 설계의 `claude_status` 시그니처("그 밖은 Other")

**답**: 비워 둔다(`None`). FR-AGENT-009대로 CLI가 방식을 알려 줄 때만 담는다. `authMethod`가 없거나 문자열이 아니면 방식 없이 로그인됨이다. plan.md `claude_status` 설명에 반영했다.

### TQ2. 단계 표시줄에서 선택 단계와 로그인 확인 단계는 언제 끝남(`done`)이 되나? 감지 중인 감지 단계의 상태는 `running`인가?

- 근거: FR-START-002("각 단계가 지금 보고 있는 단계인지, 끝났는지, 아직 시작하지 않았는지가 보인다"), `plan.md` 설계의 `indicators`(pending | running | done). 시나리오에는 감지 단계와 훅 설치 단계의 끝남만 나온다

**답**: 일이 끝나면 완료. 감지는 감지 중이 아니면, 훅 설치는 설치가 끝나면(설치 중이 아니고 결과가 있으면), 선택은 에이전트와 모델이 골라지면, 로그인 확인은 로그인됨이면 `done`이다. `done`이 아니면 지금 단계이거나 감지·설치가 진행 중일 때 `running`, 그 밖은 `pending`이다. 감지 중인 감지 단계는 `running`이다. plan.md `indicators` 설명에 반영했다.
