---
name: sdd-next
description: 이 저장소의 스펙 주도 개발(SDD)을 한 단계 진행합니다. OpenSpec 변경 폴더를 보고 지금 상태(proposal, specs, clarify, plan, 승인, implement, verify, archive)를 스크립트로 판정한 뒤, 그 상태에서 할 일 하나만 하고 다음에 무엇을 할지 알려 줍니다. 다음 단계를 몰라도 이 스킬만 반복해서 부르면 기능이 스펙부터 구현, 검증, 기준 스펙 병합까지 진행됩니다. 사용자가 "기능 만들자", "스펙 쓰자", "다음 단계", "이어서 해 줘", "clarify 다시 해 줘", "sdd", "/sdd-next"라고 하거나, 새 기능이나 요구사항 변경을 시작하거나 진행 중인 openspec/changes/ 작업을 이어 가려 할 때 반드시 이 스킬을 사용하세요. 코드만 고치는 작은 수정, 커밋, PR은 이 스킬의 범위가 아닙니다.
---

# sdd-next

SDD의 유일한 입구입니다. 부를 때마다 **상태를 판정하고, 그 상태의 일 하나만 하고, 멈춥니다.** 루프를 돌지 않습니다.

단계별 규칙은 이 파일에 없습니다. `openspec/schemas/namaste/schema.yaml`(산출물별 `instruction`, `apply.instruction`)과 `openspec/config.yaml`(`context`, `rules`)에만 있고, 아래 절차는 그 규칙을 꺼내 오는 방법만 정합니다. 규칙을 바꿀 때도 그 두 파일만 고칩니다.

OpenSpec CLI는 `pnpm exec openspec`으로 부릅니다(저장소에 버전이 고정되어 있습니다). 사용 통계를 보내지 않도록 앞에 `OPENSPEC_TELEMETRY=0`을 붙입니다.

## 1. 상태 판정

```bash
node scripts/sdd-state.mjs [<change>]
```

출력의 `state`만 보고 다음 절에서 할 일을 고릅니다. 상태를 스스로 추측하지 않습니다. `missing`은 지금 상태에서 부족한 것이고, `conflicts`는 다른 변경이나 이미 쓴 ID와 겹치는 것입니다.

- `state`가 `choose`면 진행 중인 변경이 여럿입니다. `changes` 목록을 보여 주고 어느 것을 이어 갈지 묻습니다.
- `state`가 `none`이면 진행 중인 변경이 없습니다. 새 변경을 시작합니다(아래 `none`).

## 2. 상태별로 할 일

| state | 할 일 |
| --- | --- |
| `none` | 무엇을 왜 만들지 사용자에게 듣고, 동작을 나타내는 kebab-case 이름을 정해 `openspec new change <이름>`을 실행합니다. 이어서 `proposal`을 진행합니다 |
| `proposal`, `specs`, `clarify`, `plan` | `openspec instructions <state> --change <change> --json`을 실행하고, 결과의 `instruction`, `template`, `context`, `rules`를 그대로 따라 그 산출물 하나를 씁니다. `specs`에서 `missing`이 검사 실패이면 그 문제부터 고칩니다 |
| `approval` | 사람의 승인을 받습니다. 아래 "승인" |
| `implement`, `verify` | `openspec instructions apply --change <change> --json`을 실행하고, 결과 `instruction`의 같은 이름 절(`## implement`, `## verify`)을 따릅니다 |
| `archive` | `openspec archive <change> -y`를 실행합니다. delta가 `openspec/specs/`에 합쳐지고 변경 폴더가 `openspec/changes/archive/`로 옮겨집니다. 기능 PR을 머지하기 전에 같은 브랜치에서 합니다 |

### 승인

에이전트가 혼자 승인하지 않습니다.

1. 읽을 파일을 보여 줍니다: `proposal.md`, `specs/**/spec.md`, `clarify.md`, `plan.md`.
2. 승인할지 묻습니다(Claude는 AskUserQuestion, Codex는 대화).
3. 사용자가 명시적으로 승인하면 `.openspec.yaml`에 `x-approved: <값>` 한 줄을 씁니다(이미 있으면 값을 바꿉니다). 값은 `node scripts/sdd-state.mjs <change> --hash approval`의 출력입니다. proposal, delta, plan 설계가 바뀌면 값이 달라져 다시 승인을 받습니다.
4. 고칠 점을 받으면 해당 산출물을 고치고 승인 표시는 쓰지 않습니다.

### 원할 때 clarify 다시 하기

사용자가 `sdd-next clarify`라고 부르거나 "clarify 다시 해 줘"라고 하면, 상태 판정 결과와 상관없이 clarify를 한 라운드 더 합니다. proposal 이후, archive 전에만 할 수 있습니다.

1. `openspec instructions clarify --change <change> --json`을 실행하고 결과 `instruction`의 "다시 하기" 규칙을 따릅니다.
2. 새 질문이 생기면 그 자리에서 묻습니다. 답하지 않은 질문이 남으면 다음 `sdd-next`가 자동으로 clarify 상태가 되어 다시 묻습니다(판정 순서에서 clarify가 plan, 승인, implement보다 앞이기 때문입니다).

### 앞 단계로 돌아가기

지금 단계에서 앞 단계 산출물을 고쳐야 하면 `openspec/config.yaml` `context`의 "앞 단계로 돌아가기"를 따릅니다. 사람이 "스펙부터 다시", "plan 다시"처럼 앞 단계를 고쳐 달라고 할 때도 같습니다. 그 산출물만 고치고 멈추면, 다음 `sdd-next`부터 상태 판정이 그 다음 단계로 돌아가 차례대로 다시 진행합니다.

## 3. 마치며

부를 때마다 응답의 **맨 끝**을 아래 안내로 끝냅니다. 예외가 없습니다. 질문에 답을 받다가 멈추거나, 앞 단계로 돌아가기를 묻거나, 검사가 실패해 멈춰도 똑같이 출력합니다. 먼저 `node scripts/sdd-state.mjs <change>`를 다시 실행해 새 상태를 확인합니다.

```
지금: <방금 끝낸 단계> — <방금 한 일>
다음: <새 state>. <missing을 풀어 쓴 문장>. 이어 가려면 /sdd-next를 다시 부르세요.
```

- Codex에서는 `/sdd-next` 대신 `$sdd-next`로 씁니다.
- 방금 끝낸 단계나 새 state가 clarify일 때만 이 줄을 덧붙입니다: `clarify를 한 번 더 하려면 /sdd-next clarify (Codex: $sdd-next clarify)`
- `conflicts`가 비어 있지 않으면 그 목록을 안내 바로 위에 보여 줍니다.
- 앞 단계로 돌아가기를 묻는 중이면 `다음:`에 "답에 따라 <단계>로 돌아갑니다"를 씁니다.

## 하지 않는 것

- 한 번 부를 때 두 상태 이상을 진행하지 않습니다(위 "clarify 다시 하기"도 clarify 한 라운드만 합니다). 사용자가 "끝까지"라고 해도 상태마다 결과를 보여 주고 멈춥니다.
- `openspec/specs/`를 손으로 고치지 않습니다. 기준 스펙은 archive로만 바뀝니다.
- `openspec/schemas/`, `openspec/config.yaml`을 진행 중에 고치지 않습니다. 규칙이 맞지 않으면 사용자에게 알리고, 규칙 변경은 별도 PR로 합니다.
- OpenSpec의 다른 명령(`/opsx:*`)으로 단계를 건너뛰지 않습니다.
- 커밋, push, 브랜치 이동은 하지 않습니다. 사용자가 따로 요청하면 `commit` 스킬을 씁니다.
