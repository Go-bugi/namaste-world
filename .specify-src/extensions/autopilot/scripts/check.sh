#!/usr/bin/env bash
# 단계 산출물을 읽어 판정 결과를 JSON으로 낸다. workflow 조건은 이 JSON만 본다.
#   check.sh <종류>
# 결과는 표준 출력과 <기능>/loop/<종류>.json에 쓴다. 산출물이 아예 없으면 halt.sh로 멈춘다.
source "$(dirname "$0")/lib.sh"
HERE="$(dirname "$0")"

kind="${1:?종류가 필요합니다}"

emit() {
  local json="$1" out
  out="$(loop_dir)/$kind.json"
  printf '%s\n' "$json" | tee "$out"
}

halt() { "$HERE/halt.sh" "$@"; }

# --- A. 스펙 -------------------------------------------------------------

check_specify() {
  local dir
  dir="$(feature_dir 2>/dev/null)" || halt "specify가 기능 디렉터리를 만들지 않았습니다" "specify 단계 출력을 확인합니다."
  [[ -s "$dir/spec.md" ]] || halt "$dir/spec.md가 없습니다" "specify 단계 출력을 확인합니다."
  mkdir -p "$dir/loop" "$dir/clarify"
  if [[ -f "$AUTOPILOT_SCRATCH/context.md" ]]; then
    cp "$AUTOPILOT_SCRATCH/context.md" "$dir/context.md"
  fi
  emit "$(jq -n --arg d "$dir" '{feature_dir: $d}')"
}

# 마지막 clarify 라운드 파일에서 질문 수와 해결 수를 읽어 다음 라운드를 할지 정한다.
check_clarify() {
  local dir last round questions resolved again
  dir="$(feature_dir)"
  last="$(find "$dir/clarify" -name 'round-*.md' 2>/dev/null | sort -V | tail -1)"
  if [[ -z "$last" ]]; then
    round=0 questions=0 resolved=0
  else
    round="$(basename "$last" .md)"
    round="${round#round-}"
    questions="$(count_lines '^### Q[0-9]+' "$last")"
    resolved="$(grep -oE 'autopilot: resolved=[0-9]+' "$last" | tail -1 | cut -d= -f2 || true)"
    resolved="${resolved:-0}"
  fi
  again=false
  if ((questions > 0 && resolved > 0 && round < 3)); then again=true; fi
  emit "$(jq -n --argjson r "$round" --argjson q "$questions" --argjson s "$resolved" --argjson a "$again" \
    '{round: $r, questions: $q, resolved: $s, again: $a}')"
}

# 질문 파일이 막 만들어졌는지(이번 라운드에 사람이 답할 것이 있는지).
check_clarify_ask() {
  local dir last questions
  dir="$(feature_dir)"
  last="$(find "$dir/clarify" -name 'round-*.md' 2>/dev/null | sort -V | tail -1)"
  questions=0
  if [[ -n "$last" ]] && ! grep -q 'autopilot: resolved=' "$last"; then
    questions="$(count_lines '^### Q[0-9]+' "$last")"
  fi
  emit "$(jq -n --arg f "${last:-}" --argjson q "$questions" '{file: $f, questions: $q}')"
}

check_checklist() {
  local file fixable decision
  file="$(feature_dir)/checklists/testability.md"
  [[ -f "$file" ]] || halt "checklist가 ${file}을 만들지 않았습니다" "checklist 단계 출력을 확인합니다."
  decision="$(grep -E '^- \[ \]' "$file" | grep -cE '\[결정 필요\]' || true)"
  fixable="$(grep -E '^- \[ \]' "$file" | grep -E '\[(Gap|Ambiguity|Conflict)\]' | grep -cvE '\[결정 필요\]' || true)"
  emit "$(jq -n --argjson f "$fixable" --argjson d "$decision" '{fixable: $f, decision: $d, again: ($f > 0)}')"
}

check_conflicts() {
  local file contradictions replacements decisions
  file="$(feature_dir)/conflicts.md"
  [[ -f "$file" ]] || halt "충돌 검사가 ${file}을 만들지 않았습니다" "conflicts 단계 출력을 확인합니다."
  contradictions="$(count_lines '^- \[모순\]' "$file")"
  replacements="$(count_lines '^- \[대체\]' "$file")"
  # [결정]은 spec을 고쳐서 풀 수 없고 GATE-1에서 사람이 정하므로 재시도하지 않는다.
  decisions="$(count_lines '^- \[결정\]' "$file")"
  emit "$(jq -n --argjson c "$contradictions" --argjson r "$replacements" --argjson d "$decisions" \
    '{contradictions: $c, replacements: $r, decisions: $d, again: ($c > 0)}')"
}

# GATE-1에서 사람이 볼 요약.
check_gate1() {
  local dir out
  dir="$(feature_dir)"
  out="$dir/loop/gate1.md"
  {
    echo "# GATE-1 스펙 승인"
    echo
    echo "- spec: $dir/spec.md"
    echo "- 남은 clarify 질문: $(jq -r '.questions - .resolved' "$dir/loop/clarify.json" 2>/dev/null || echo 0)"
    echo "- checklist 고칠 항목: $(jq -r .fixable "$dir/loop/checklist.json" 2>/dev/null || echo '?'), 결정 필요: $(jq -r .decision "$dir/loop/checklist.json" 2>/dev/null || echo '?') ($dir/checklists/testability.md)"
    echo "- 기존 스펙과 모순: $(jq -r .contradictions "$dir/loop/conflicts.json" 2>/dev/null || echo '?'), 의도된 대체: $(jq -r .replacements "$dir/loop/conflicts.json" 2>/dev/null || echo '?'), 결정 필요: $(jq -r '.decisions // 0' "$dir/loop/conflicts.json" 2>/dev/null || echo '?') ($dir/conflicts.md)"
    echo
    echo "## 고르기"
    echo
    echo "- approve: 이 spec으로 설계와 구현을 시작합니다."
    echo "- edit: spec.md를 직접 고쳤습니다. checklist와 충돌 검사를 다시 돌리고 이 화면으로 돌아옵니다."
    echo "- reject: $dir/loop/gate1-feedback.md에 고칠 점을 적었습니다. 에이전트가 spec을 고친 뒤 이 화면으로 돌아옵니다."
    echo
    echo "edit와 reject는 합쳐 3번까지입니다. 그 뒤에는 ABORT합니다."
    if [[ -f "$dir/conflicts.md" ]]; then
      echo
      echo "## conflicts.md"
      echo
      sed -n '1,80p' "$dir/conflicts.md"
    fi
  } >"$out"
  emit "$(jq -n --arg f "$out" '{file: $f}')"
}

# --- B. 설계와 구현 --------------------------------------------------------

check_plan() {
  local dir
  dir="$(feature_dir)"
  [[ -s "$dir/plan.md" ]] || halt "plan.md가 없습니다" "plan 단계 출력을 확인합니다."
  [[ -d "$dir/contracts" && -n "$(ls -A "$dir/contracts")" ]] ||
    halt "contracts/가 비어 있습니다" "plan이 테스트할 공개 함수의 시그니처를 contracts/에 쓰지 않았습니다. plan을 고친 뒤 새로 실행합니다."
  emit "$(jq -n --argjson n "$(find "$dir/contracts" -type f | wc -l)" '{contracts: $n}')"
}

check_tasks() {
  local file p missing=()
  file="$(feature_dir)/tasks.md"
  [[ -s "$file" ]] || halt "tasks.md가 없습니다" "tasks 단계 출력을 확인합니다."
  for p in 1 2 3 4 5; do
    grep -qE "^## Phase $p" "$file" || missing+=("$p")
  done
  ((${#missing[@]} == 0)) || halt "tasks.md에 Phase ${missing[*]}이(가) 없습니다" "namaste preset의 tasks 템플릿 순서대로 다시 만듭니다."
  emit "$(jq -n --argjson n "$(count_lines '^- \[ \]' "$file")" '{open_tasks: $n}')"
}

# analyze 결과의 CRITICAL, HIGH 수. 직전 기록과 비교해 진전 없음과 퇴행을 판정한다.
check_analyze() {
  local dir report history critical high prev_total total hash prev_hash stalled=false regressed=false
  dir="$(feature_dir)"
  report="$dir/loop/analyze.md"
  [[ -f "$report" ]] || halt "analyze 보고서($report)가 없습니다" "analyze 단계 출력을 확인합니다."
  critical="$(parse_count critical "$report")"
  high="$(parse_count high "$report")"
  [[ -n "$critical" && -n "$high" ]] || halt "analyze 보고서에 COUNTS 줄이 없습니다" "$report 형식을 확인합니다."
  total=$((critical + high))
  history="$dir/loop/analyze.history"
  prev_total="$(last_record "$history" total)"
  prev_hash="$(last_record "$history" hash)"
  hash="$(worktree_hash)"
  if [[ -n "$prev_total" ]] && ((total > 0)); then
    ((total > prev_total)) && regressed=true
    [[ "$hash" == "$prev_hash" ]] && stalled=true
  fi
  record_history "$history" "$(jq -cn --argjson t "$total" --arg h "$hash" '{total: $t, hash: $h}')"
  emit "$(jq -n --argjson c "$critical" --argjson h "$high" --argjson s "$stalled" --argjson r "$regressed" \
    '{critical: $c, high: $h, pass: ($c + $h == 0), stalled: $s, regressed: $r,
      again: (($c + $h > 0) and ($s | not) and ($r | not))}')"
}

# B4-1 뒤: 시그니처 파일이 컴파일되는가.
check_model() {
  local log open
  log="$(loop_dir)/model.log"
  # 아무 파일도 만들지 않아도 컴파일은 통과하므로 Phase 1 작업이 모두 체크됐는지 먼저 본다.
  open="$(count_open_tasks 1 "$(feature_dir)/tasks.md")"
  [[ "$open" == 0 ]] || halt "B4-1 모델링이 끝나지 않았습니다(Phase 1 미완료 작업 ${open}개)" "model 단계 출력(workflow 실행 상태의 stdout)에서 멈춘 이유를 확인합니다."
  if pnpm -s typecheck >"$log" 2>&1 && (cd src-tauri && cargo check --quiet --tests) >>"$log" 2>&1; then
    emit '{"compiles": true}'
  else
    halt "B4-1 모델링 결과가 컴파일되지 않습니다" "${log}를 보고 contracts와 시그니처를 맞춥니다."
  fi
}

# B4-2 뒤: 바뀐 파일이 테스트 범위 안인가, 테스트가 컴파일되고 실패하는가(red).
# 범위 기준은 bundle.sh가 테스트 작성 직전에 남긴 snapshot(경로, 내용 해시, Rust 구현부 해시)이다.
# 고정할 테스트 목록은 snapshot과 상관없이 "HEAD 대비 바뀐 테스트"로 다시 모은다(재작성 회차에도 앞 회차 테스트를 포함).
check_tests() {
  local dir loop snap f entry bad=() test_files new_tests compiles=true red=false fail=false log
  dir="$(feature_dir)"
  loop="$dir/loop"
  snap="$loop/pre-tests.txt"
  [[ -f "$snap" ]] || die "pre-tests.txt가 없습니다. bundle.sh가 먼저 돌아야 합니다."

  # 1) 범위: 이번 시도에서 바뀐 파일이 테스트, 질문 파일, loop 기록뿐인가.
  while IFS= read -r f; do
    [[ -z "$f" ]] && continue
    entry="$(awk -v p="$f" '$3 == p' "$snap")"
    [[ -n "$entry" && "$(sha <"$f" 2>/dev/null || echo gone)" == "$(cut -d' ' -f1 <<<"$entry")" ]] && continue
    case "$f" in
      *.test.ts | *.test.tsx | "$dir"/test-questions.md | "$dir"/test-disputes.md | "$dir"/loop/*) ;;
      *.rs) [[ -f "$f" && "$(rust_head_hash "$f")" == "$(baseline_rust_head "$f" "$entry")" ]] || bad+=("$f") ;;
      *) bad+=("$f") ;;
    esac
  done <<<"$(changed_files)"
  ((${#bad[@]} == 0)) || restore_files "$loop/pre-snapshot" "${bad[@]}"

  # 2) 고정할 테스트 목록.
  test_files="$loop/test-files.txt"
  changed_tests >"$test_files"
  new_tests="$(grep -c . "$test_files" || true)"

  # 3) red: 컴파일은 되고 테스트는 실패한다.
  log="$loop/tests.log"
  : >"$log"
  pnpm -s typecheck >>"$log" 2>&1 || compiles=false
  (cd src-tauri && cargo test --no-run --quiet) >>"$log" 2>&1 || compiles=false
  if [[ "$compiles" == true ]]; then
    pnpm -s test >>"$log" 2>&1 || fail=true
    (cd src-tauri && cargo test --quiet) >>"$log" 2>&1 || fail=true
    if ((new_tests > 0 && ${#bad[@]} == 0)) && [[ "$fail" == true ]]; then red=true; fi
  fi
  [[ "$red" == true ]] && test_fingerprint "$test_files" >"$loop/test-hash.txt"
  emit "$(jq -n --argjson n "$new_tests" --argjson c "$compiles" --argjson r "$red" \
    --argjson b "$(json_array "${bad[@]+"${bad[@]}"}")" \
    '{new_test_files: $n, compiles: $c, red: $r, out_of_scope: $b, again: ($r | not)}')"
}

# HEAD 대비 바뀐 테스트: TS 테스트 파일, 테스트 모듈이 바뀐 Rust 파일.
changed_tests() {
  local f
  while IFS= read -r f; do
    [[ -f "$f" ]] || continue
    case "$f" in
      specs/*/loop/*) ;; # 입력 묶음과 snapshot에 든 사본
      *.test.ts | *.test.tsx) echo "$f" ;;
      *.rs)
        grep -q '#\[cfg(test)\]' "$f" || continue
        [[ "$(sed -n '/#\[cfg(test)\]/,$p' "$f" | sha)" != "$(git show "HEAD:$f" 2>/dev/null | sed -n '/#\[cfg(test)\]/,$p' | sha)" ]] && echo "$f"
        ;;
    esac
  done <<<"$(changed_files)"
}

# Rust 파일의 테스트 모듈 앞부분 기준값. 테스트 작성 전에 바뀐 파일이면 snapshot 값, 아니면 HEAD의 값.
baseline_rust_head() {
  local f="$1" entry="$2"
  if [[ -n "$entry" ]]; then
    cut -d' ' -f2 <<<"$entry"
  else
    git show "HEAD:$f" 2>/dev/null | sed '/#\[cfg(test)\]/,$d' | sha
  fi
}

# B6: verify.sh 결과와 리뷰 결과를 합쳐 판정한다.
check_verify() {
  local dir loop v review issues failures hash prev_count prev_hash stalled=false regressed=false pass
  dir="$(feature_dir)"
  loop="$dir/loop"
  v="$loop/verify-run.json"
  [[ -f "$v" ]] || die "verify-run.json이 없습니다. verify.sh가 먼저 돌아야 합니다."
  if [[ "$(jq -r .tamper "$v")" == true ]]; then
    halt "테스트 변조: B4 이후 테스트 파일이 바뀌었습니다" "구현 쪽 에이전트가 테스트를 고쳤습니다. git diff로 테스트 변경을 확인하고 되돌린 뒤 새로 실행합니다."
  fi
  failures="$(jq -r .count "$v")"
  issues=0
  review="$loop/review.md"
  if ((failures == 0)); then
    [[ -f "$review" ]] || halt "리뷰 보고서($review)가 없습니다" "review 단계 출력을 확인합니다."
    issues="$(parse_count issues "$review")"
    [[ -n "$issues" ]] || halt "리뷰 보고서에 COUNTS 줄이 없습니다" "$review 형식을 확인합니다."
  fi
  hash="$(jq -r .error_hash "$v")-$(worktree_hash)"
  prev_count="$(last_record "$loop/verify.history" total)"
  prev_hash="$(last_record "$loop/verify.history" hash)"
  if [[ -n "$prev_count" ]]; then
    ((failures + issues > prev_count)) && regressed=true
    [[ "$hash" == "$prev_hash" ]] && stalled=true
  fi
  pass=false
  ((failures == 0 && issues == 0)) && pass=true
  [[ "$pass" == true ]] && stalled=false && regressed=false
  record_history "$loop/verify.history" "$(jq -cn --argjson t $((failures + issues)) --arg h "$hash" '{total: $t, hash: $h}')"
  emit "$(jq -n --argjson f "$failures" --argjson i "$issues" --argjson p "$pass" --argjson s "$stalled" --argjson r "$regressed" \
    --argjson names "$(jq .failures "$v")" \
    '{failures: $f, failed_checks: $names, review_issues: $i, pass: $p, stalled: $s, regressed: $r,
      again: (($p | not) and ($s | not) and ($r | not))}')"
}

# B7: converge가 빠진 작업을 추가했는지. 열린 작업이 없으면 Converged.
check_converge() {
  local open
  open="$(count_lines '^- \[ \]' "$(feature_dir)/tasks.md")"
  emit "$(jq -n --argjson o "$open" '{open_tasks: $o, converged: ($o == 0), again: ($o > 0)}')"
}

# --- 공통 -----------------------------------------------------------------

changed_files() {
  { git diff --name-only HEAD; git ls-files --others --exclude-standard; } | sort -u
}

# 범위를 벗어난 파일을 테스트 작성 직전 상태로 되돌린다.
restore_files() {
  local snapdir="$1" f
  shift
  for f in "$@"; do
    if [[ -f "$snapdir/$f" ]]; then
      cp "$snapdir/$f" "$f"
    elif git ls-files --error-unmatch "$f" >/dev/null 2>&1; then
      git checkout -- "$f"
    else
      rm -f "$f"
    fi
  done
}

json_array() {
  if (($# == 0)); then echo '[]'; else printf '%s\n' "$@" | jq -R . | jq -sc .; fi
}

case "$kind" in
  specify) check_specify ;;
  clarify-ask) check_clarify_ask ;;
  clarify) check_clarify ;;
  checklist) check_checklist ;;
  conflicts) check_conflicts ;;
  gate1) check_gate1 ;;
  plan) check_plan ;;
  tasks) check_tasks ;;
  analyze) check_analyze ;;
  model) check_model ;;
  tests) check_tests ;;
  verify) check_verify ;;
  converge) check_converge ;;
  *) die "알 수 없는 종류: $kind" ;;
esac
