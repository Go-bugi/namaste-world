#!/usr/bin/env bash
# 에이전트 대신 단계 산출물을 만드는 픽스처. 시나리오는 SCENARIO 환경 변수로 고른다.
#   agent.sh <step-id>
# 호출마다 .fixture/calls에 step-id를 한 줄 남긴다(루프 횟수를 세는 데 씀).
set -euo pipefail
step="$1"
scenario="${SCENARIO:-happy}"
mkdir -p .fixture
echo "$step" >>.fixture/calls
n="$(grep -cx "$step" .fixture/calls)"
F=specs/001-demo

bump() { echo "change $step $n $RANDOM" >>"$1"; }

case "$step" in
  context)
    mkdir -p .specify/autopilot
    printf '# Context\n\n- 기존 기능 없음\n' >.specify/autopilot/context.md
    ;;
  specify)
    mkdir -p "$F"
    printf '{"feature_directory": "%s"}\n' "$F" >.specify/feature.json
    cat >"$F/spec.md" <<'SPEC'
# Feature Specification: demo

## User Scenarios & Testing

### User Story 1 - demo (Priority: P1)

**Acceptance Scenarios**:

1. **Given** a, **When** b, **Then** c

## Requirements

### Functional Requirements

- **FR-DEMO-001**: System MUST do c
SPEC
    ;;
  clarify-ask)
    r="$(find "$F/clarify" -name 'round-*.md' 2>/dev/null | wc -l | tr -d ' ')"
    r=$((r + 1))
    if [[ "$scenario" == clarify-forever ]]; then
      printf '# Clarify round %s\n\n### Q1: q\n\n- **답**:\n' "$r" >"$F/clarify/round-$r.md"
    else
      printf '# Clarify round %s\n\n질문 없음\n<!-- autopilot: resolved=0 -->\n' "$r" >"$F/clarify/round-$r.md"
    fi
    ;;
  clarify-apply)
    last="$(find "$F/clarify" -name 'round-*.md' | sort -V | tail -1)"
    echo '<!-- autopilot: resolved=1 -->' >>"$last"
    ;;
  checklist | gate1-checklist)
    mkdir -p "$F/checklists"
    if [[ "$scenario" == checklist-forever ]]; then
      printf -- '- [ ] CHK001 [Gap] x\n' >"$F/checklists/testability.md"
    else
      printf -- '- [ ] CHK001 ok\n' >"$F/checklists/testability.md"
    fi
    ;;
  conflicts | gate1-conflicts)
    if [[ "$scenario" == conflicts-forever ]]; then
      printf -- '- [모순] FR-DEMO-001 vs FR-OLD-001 (memory): x\n' >"$F/conflicts.md"
    else
      printf -- '- 없음\n' >"$F/conflicts.md"
    fi
    ;;
  checklist-fix | conflicts-fix | gate1-fix) bump "$F/spec.md" ;;
  plan)
    mkdir -p "$F/contracts"
    echo '# plan' >"$F/plan.md"
    echo '# demo contract' >"$F/contracts/demo.md"
    ;;
  tasks)
    printf '## Phase 1\n- [ ] T001 [core] model\n## Phase 2\n- [ ] T002 test\n## Phase 3\n- [ ] T003 core\n## Phase 4\n## Phase 5\n' >"$F/tasks.md"
    ;;
  analyze | final-analyze)
    mkdir -p "$F/loop"
    case "$scenario" in
      analyze-forever | analyze-stalled) c=1 ;;
      analyze-regress) c="$n" ;;
      *) c=0 ;;
    esac
    [[ "$step" == final-analyze ]] && c=0
    printf '# report\n\nCOUNTS: critical=%s high=0 medium=0 low=0\n' "$c" >"$F/loop/analyze.md"
    ;;
  analyze-fix) [[ "$scenario" == analyze-stalled ]] || bump "$F/plan.md" ;;
  model)
    sed -i.bak 's/- \[ \] T001/- [x] T001/' "$F/tasks.md" && rm -f "$F/tasks.md.bak"
    mkdir -p src/demo
    echo "export const demo = (): number => { throw new Error('not implemented') }" >src/demo/demo.ts
    ;;
  test-write)
    if [[ "$scenario" == tests-out-of-scope ]]; then bump src/demo/demo.ts; fi
    echo "test('FR-DEMO-001', () => {})" >src/demo/demo.test.ts
    ;;
  implement)
    sed -i.bak 's/- \[ \]/- [x]/' "$F/tasks.md" && rm -f "$F/tasks.md.bak"
    echo "export const demo = (): number => 1" >src/demo/demo.ts
    ;;
  review)
    mkdir -p "$F/loop"
    printf '# Review\n\nCOUNTS: issues=0\n' >"$F/loop/review.md"
    ;;
  verify-fix)
    case "$scenario" in
      verify-stalled) ;;
      tamper) echo "// tampered" >>src/demo/demo.test.ts ;;
      *) bump src/demo/demo.ts ;;
    esac
    ;;
  converge)
    if [[ "$scenario" == converge-forever ]]; then
      printf -- '- [ ] T9%s more\n' "$n" >>"$F/tasks.md"
    fi
    ;;
  gate1-polish | gate1-repolish | gate2-polish) ;;
  *) echo "fixture: 모르는 단계 $step" >&2; exit 1 ;;
esac
