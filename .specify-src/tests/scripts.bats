#!/usr/bin/env bats
# autopilot 판정 스크립트 단위 테스트. 픽스처 산출물을 넣고 결과 JSON을 비교한다.

load helpers

setup() {
  make_repo ""
  make_feature
  S=.specify/extensions/autopilot/scripts
}

json() { jq -r "$1" <<<"$output"; }

# 진입: 커밋하지 않은 변경이 있으면 거부
@test "entry rejects dirty tree" {
  echo x >dirty.txt
  run bash $S/entry.sh
  [ "$status" -eq 1 ]
  [[ "$output" == *"커밋하지 않은 변경"* ]]
}

# clarify: 해결된 답이 있고 3라운드 전이면 다음 라운드
@test "clarify continues while answers resolve and round < 3" {
  printf '### Q1: a\n### Q2: b\n<!-- autopilot: resolved=1 -->\n' >"$F/clarify/round-1.md"
  run bash $S/check.sh clarify
  [ "$(json .again)" = true ]
  [ "$(json .questions)" = 2 ]
}

# clarify: 한 라운드에서 해결된 답이 0개면 멈춤
@test "clarify stops when a round resolves nothing" {
  printf '### Q1: a\n<!-- autopilot: resolved=0 -->\n' >"$F/clarify/round-1.md"
  run bash $S/check.sh clarify
  [ "$(json .again)" = false ]
}

# clarify: 3라운드면 해결된 답이 있어도 멈춤
@test "clarify stops at round 3" {
  for r in 1 2 3; do printf '### Q1: a\n<!-- autopilot: resolved=1 -->\n' >"$F/clarify/round-$r.md"; done
  run bash $S/check.sh clarify
  [ "$(json .round)" = 3 ]
  [ "$(json .again)" = false ]
}

# checklist: [결정 필요]가 붙은 항목은 고칠 항목에서 빠지고, 체크된 항목은 세지 않음
@test "checklist counts fixable and decision items separately" {
  printf -- '- [ ] CHK1 [Gap] a\n- [ ] CHK2 [Ambiguity] b [결정 필요]\n- [x] CHK3 [Gap] c\n- [ ] CHK4 plain\n' >"$F/checklists/testability.md"
  run bash $S/check.sh checklist
  [ "$(json .fixable)" = 1 ]
  [ "$(json .decision)" = 1 ]
  [ "$(json .again)" = true ]
}

# checklist: 파일이 없으면 HALT
@test "checklist halts when the file is missing" {
  run bash $S/check.sh checklist
  [ "$status" -eq 1 ]
  [ -f "$F/loop/halt.md" ]
}

# conflicts: 모순과 대체를 따로 셈
@test "conflicts counts contradictions and replacements" {
  printf -- '- [모순] FR-A vs FR-B (memory): x\n- [대체] FR-C → FR-D (memory)\n' >"$F/conflicts.md"
  run bash $S/check.sh conflicts
  [ "$(json .contradictions)" = 1 ]
  [ "$(json .replacements)" = 1 ]
}

# conflicts: [결정]은 GATE-1로 넘기고 spec 수정을 다시 돌리지 않음
@test "conflicts sends decisions to gate1 without retry" {
  printf -- '- [결정] Domain NEW: constitution 표에 없음\n' >"$F/conflicts.md"
  run bash $S/check.sh conflicts
  [ "$(json .decisions)" = 1 ]
  [ "$(json .again)" = false ]
}

# model: Phase 1 작업이 남아 있으면 컴파일과 상관없이 HALT
@test "model halts when phase 1 tasks are open" {
  printf '## Phase 1: Modeling\n- [x] T001 a\n- [ ] T002 b\n## Phase 2: Tests\n- [ ] T003 c\n' >"$F/tasks.md"
  run bash $S/check.sh model
  [ "$status" -eq 1 ]
  grep -q 'Phase 1 미완료 작업 1개' "$F/loop/halt.md"
}

# analyze: CRITICAL, HIGH가 0이면 통과
@test "analyze passes with zero critical and high" {
  printf 'x\nCOUNTS: critical=0 high=0 medium=3 low=1\n' >"$F/loop/analyze.md"
  run bash $S/check.sh analyze
  [ "$(json .pass)" = true ]
  [ "$(json .again)" = false ]
}

# analyze: 같은 작업 트리로 두 번째 실패면 진전 없음
@test "analyze detects no progress" {
  printf 'COUNTS: critical=1 high=0 medium=0 low=0\n' >"$F/loop/analyze.md"
  bash $S/check.sh analyze >/dev/null
  run bash $S/check.sh analyze
  [ "$(json .stalled)" = true ]
  [ "$(json .again)" = false ]
}

# analyze: COUNTS 줄이 없으면 HALT
@test "analyze halts without a COUNTS line" {
  echo 'no counts' >"$F/loop/analyze.md"
  run bash $S/check.sh analyze
  [ "$status" -eq 1 ]
}

# converge: 열린 작업이 없으면 Converged
@test "converge reports converged when no open tasks remain" {
  printf -- '- [x] T1\n- [X] T2\n' >"$F/tasks.md"
  run bash $S/check.sh converge
  [ "$(json .converged)" = true ]
}

# tasks: Phase 1~5가 모두 있어야 함
@test "tasks halts when a phase is missing" {
  printf '## Phase 1\n## Phase 2\n## Phase 3\n## Phase 5\n' >"$F/tasks.md"
  run bash $S/check.sh tasks
  [ "$status" -eq 1 ]
  grep -q 'Phase 4' "$F/loop/halt.md"
}

# plan: contracts/가 비어 있으면 HALT
@test "plan halts with empty contracts" {
  echo '# plan' >"$F/plan.md"
  run bash $S/check.sh plan
  [ "$status" -eq 1 ]
  grep -q 'contracts' "$F/loop/halt.md"
}

# verify: 테스트 해시가 바뀌면 변조로 HALT
@test "verify halts on tampered tests" {
  mkdir -p src
  echo 'test a' >src/a.test.ts
  echo src/a.test.ts >"$F/loop/test-files.txt"
  source $S/lib.sh
  test_fingerprint "$F/loop/test-files.txt" >"$F/loop/test-hash.txt"
  echo 'changed' >>src/a.test.ts
  run bash $S/verify.sh
  [ "$(json .tamper)" = true ]
  run bash $S/check.sh verify
  [ "$status" -eq 1 ]
  grep -q '변조' "$F/loop/halt.md"
}

# verify: Rust 테스트 모듈 앞부분(구현)은 바뀌어도 변조가 아님
@test "rust fingerprint covers only the test module" {
  source $S/lib.sh
  printf 'fn a() {}\n#[cfg(test)]\nmod tests {}\n' >x.rs
  echo x.rs >list.txt
  before="$(test_fingerprint list.txt)"
  sed -i.bak 's/fn a() {}/fn a() { 1; }/' x.rs
  [ "$(test_fingerprint list.txt)" = "$before" ]
  sed -i.bak 's/mod tests {}/mod tests { }/' x.rs
  [ "$(test_fingerprint list.txt)" != "$before" ]
}

# audit: 모든 FR에 Source가 있으면 통과
@test "audit passes when every FR has a source ref" {
  printf -- '- a [Source: %s/spec.md -> FR-DEMO-001]\n- b [Source: %s/spec.md -> FR-DEMO-002]\n' "$F" "$F" >>.specify/memory/spec.md
  run bash $S/audit.sh
  [ "$(json .pass)" = true ]
}

# audit: changelog에 그 기능과 ID가 있으면(보류, 미해결 모순) 빠진 것으로 보지 않음
@test "audit accepts ids recorded in the changelog" {
  printf -- '- a [Source: %s/spec.md -> FR-DEMO-001]\n' "$F" >>.specify/memory/spec.md
  printf -- '- FR-DEMO-002 withheld. Raised by %s\n' "$F" >>.specify/memory/changelog.md
  run bash $S/audit.sh
  [ "$(json .pass)" = true ]
}

# audit: 빠진 ID를 알려 줌
@test "audit lists missing ids" {
  printf -- '- a [Source: %s/spec.md -> FR-DEMO-001]\n' "$F" >>.specify/memory/spec.md
  run bash $S/audit.sh
  [ "$(json .pass)" = false ]
  [ "$(json '.missing[0]')" = FR-DEMO-002 ]
}

# halt: 이유와 할 일을 halt.md에 쓰고 실패로 끝남
@test "halt writes reason and exits 1" {
  echo '{"x": 1}' >"$F/loop/demo.json"
  run bash $S/halt.sh "이유 테스트" "할 일 테스트"
  [ "$status" -eq 1 ]
  grep -q '이유 테스트' "$F/loop/halt.md"
  grep -q '할 일 테스트' "$F/loop/halt.md"
  grep -q 'demo.json' "$F/loop/halt.md"
}
