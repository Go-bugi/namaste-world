#!/usr/bin/env bats
# 루프가 반드시 끝나는지 확인한다. 실제 workflow.yml의 에이전트 단계만 픽스처로 바꿔서 돌린다.
# 각 시나리오가 정해진 상한(또는 진전 없음, 퇴행, 변조)에서 HALT하고 halt.md에 이유를 남기는지 본다.

load helpers

OK_REF='- **FR-1**: c [Source: specs/001-demo/spec.md -> FR-DEMO-001]'
APPROVE=(--input gate1_verdict=approve --input gate2_verdict=done)

# 정상 흐름: 모든 검사를 통과하면 DONE
@test "happy path reaches DONE" {
  make_repo "$OK_REF"
  run_loop happy "${APPROVE[@]}"
  [ "$STATUS" = completed ] || { echo "$output"; false; }
  [ ! -f specs/001-demo/loop/halt.md ]
  [ "$(calls implement)" -eq 1 ]
  [ "$(calls review)" -eq 1 ]
}

# 진입 조건: feature/* 브랜치가 아니면 시작하지 않는다
@test "entry refuses non-feature branch" {
  make_repo "$OK_REF"
  git checkout -qb main
  run_loop happy "${APPROVE[@]}"
  [ "$STATUS" = failed ]
  [ "$(calls context)" -eq 0 ]
}

# clarify: 매 라운드 답이 있어도 3라운드에서 멈추고 다음 단계로 간다
@test "clarify stops after 3 rounds and moves on" {
  make_repo "$OK_REF"
  run_loop clarify-forever "${APPROVE[@]}"
  [ "$(calls clarify-ask)" -eq 3 ]
  [ "$(calls clarify-apply)" -eq 3 ]
  [ "$STATUS" = completed ]
}

# checklist: 고칠 항목이 계속 남아도 2회에서 멈추고 GATE-1로 간다
@test "checklist stops after 2 runs and moves on" {
  make_repo "$OK_REF"
  run_loop checklist-forever "${APPROVE[@]}"
  [ "$(calls checklist)" -eq 2 ]
  [ "$STATUS" = completed ]
}

# conflicts: 모순이 계속 남아도 2회에서 멈추고 GATE-1로 간다
@test "conflicts stops after 2 runs and moves on" {
  make_repo "$OK_REF"
  run_loop conflicts-forever "${APPROVE[@]}"
  [ "$(calls conflicts)" -eq 2 ]
  [ "$STATUS" = completed ]
}

# GATE-1: 3번 거절하면 ABORT
@test "gate1 aborts after 3 rejections" {
  make_repo "$OK_REF"
  run_loop happy --input gate1_verdict=reject --input gate2_verdict=done
  [ "$STATUS" = failed ]
  [ "$(calls gate1-fix)" -eq 3 ]
  [ "$(calls plan)" -eq 0 ]
  [[ "$(halt_reason)" == *ABORT* ]]
}

# analyze: 진전이 있어도 CRITICAL이 남으면 3회에서 HALT
@test "analyze halts after 3 runs" {
  make_repo "$OK_REF"
  run_loop analyze-forever "${APPROVE[@]}"
  [ "$STATUS" = failed ]
  [ "$(calls analyze)" -eq 3 ]
  [ "$(calls model)" -eq 0 ]
  [[ "$(halt_reason)" == *analyze* ]]
}

# analyze: 고쳐도 아무것도 바뀌지 않으면(진전 없음) 2회에서 HALT
@test "analyze halts when stalled" {
  make_repo "$OK_REF"
  run_loop analyze-stalled "${APPROVE[@]}"
  [ "$STATUS" = failed ]
  [ "$(calls analyze)" -eq 2 ]
  grep -q '"stalled": true' specs/001-demo/loop/analyze.json
}

# analyze: CRITICAL이 늘어나면(퇴행) 2회에서 HALT
@test "analyze halts on regression" {
  make_repo "$OK_REF"
  run_loop analyze-regress "${APPROVE[@]}"
  [ "$STATUS" = failed ]
  [ "$(calls analyze)" -eq 2 ]
  grep -q '"regressed": true' specs/001-demo/loop/analyze.json
}

# 테스트: 본문 없이도 통과하는 테스트면 2회 다시 쓰고 HALT
@test "tests halt when never red" {
  make_repo "$OK_REF"
  run_loop tests-never-red "${APPROVE[@]}"
  [ "$STATUS" = failed ]
  [ "$(calls test-write)" -eq 2 ]
  [ "$(calls implement)" -eq 0 ]
  [[ "$(halt_reason)" == *red* ]]
}

# 테스트: test-writer가 테스트 밖 파일을 고치면 되돌리고 실패로 센다
@test "tests revert out-of-scope edits" {
  make_repo "$OK_REF"
  run_loop tests-out-of-scope "${APPROVE[@]}"
  [ "$STATUS" = failed ]
  [ "$(calls test-write)" -eq 2 ]
  grep -q 'src/demo/demo.ts' specs/001-demo/loop/tests.json
  grep -q "not implemented" src/demo/demo.ts
}

# 검증: 진전이 있어도 계속 실패하면 5회에서 HALT
@test "verify halts after 5 runs" {
  make_repo "$OK_REF"
  run_loop verify-forever "${APPROVE[@]}"
  [ "$STATUS" = failed ]
  [ "$(calls verify-fix)" -eq 5 ]
  [ "$(calls converge)" -eq 0 ]
  [[ "$(halt_reason)" == *검증* ]]
}

# 검증: 고쳐도 같은 오류에 같은 코드면(진전 없음) 2회에서 HALT
@test "verify halts when stalled" {
  make_repo "$OK_REF"
  run_loop verify-stalled "${APPROVE[@]}"
  [ "$STATUS" = failed ]
  [ "$(calls verify-fix)" -eq 1 ]
  grep -q '"stalled": true' specs/001-demo/loop/verify.json
}

# 검증: 구현 쪽이 테스트를 고치면(변조) 즉시 HALT
@test "verify halts on test tampering" {
  make_repo "$OK_REF"
  run_loop tamper "${APPROVE[@]}"
  [ "$STATUS" = failed ]
  [ "$(calls verify-fix)" -eq 1 ]
  [[ "$(halt_reason)" == *변조* ]]
}

# converge: 매번 작업이 추가되면 3회에서 HALT
@test "converge halts after 3 rounds" {
  make_repo "$OK_REF"
  run_loop converge-forever "${APPROVE[@]}"
  [ "$STATUS" = failed ]
  [ "$(calls implement)" -eq 3 ]
  [ "$(calls final-analyze)" -eq 0 ]
  [[ "$(halt_reason)" == *converge* ]]
}

# 병합 감사: memory에 FR이 없으면 GATE-2를 3번 반복하고 HALT
@test "audit halts after 3 gate2 rounds" {
  make_repo ""
  run_loop happy "${APPROVE[@]}"
  [ "$STATUS" = failed ]
  [ "$(grep -c '"missing"' specs/001-demo/loop/audit.json)" -eq 1 ]
  [[ "$(halt_reason)" == *감사* ]]
}
