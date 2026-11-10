#!/usr/bin/env bash
# B6: code-writer "완료 기준"과 같은 검사를 같은 순서로 돌린다(Rust 먼저, TS 다음).
# 모든 검사를 끝까지 돌려 실패 목록을 한 번에 모은다. 결과는 <기능>/loop/verify-run.json.
source "$(dirname "$0")/lib.sh"

CHECKS=(test:rust lint:rust format:check:rust test lint format:check build)

loop="$(loop_dir)"
logs="$loop/verify"
rm -rf "$logs"
mkdir -p "$logs"

failures=()
for c in "${CHECKS[@]}"; do
  log="$logs/${c//:/-}.log"
  if ! pnpm -s "$c" >"$log" 2>&1; then
    failures+=("$c")
  fi
done

# 실패한 검사 로그의 끝부분으로 오류 서명을 만든다. 같은 오류가 반복되면 같은 값이 나온다.
error_hash="none"
if ((${#failures[@]})); then
  error_hash="$(for c in "${failures[@]}"; do echo "== $c"; tail -40 "$logs/${c//:/-}.log"; done | error_signature)"
fi

tamper=false
if [[ -f "$loop/test-hash.txt" ]]; then
  [[ "$(test_fingerprint "$loop/test-files.txt")" == "$(cat "$loop/test-hash.txt")" ]] || tamper=true
fi

json="$(jq -n --argjson f "$(if ((${#failures[@]})); then printf '%s\n' "${failures[@]}" | jq -R . | jq -sc .; else echo '[]'; fi)" \
  --arg h "$error_hash" --argjson t "$tamper" \
  '{failures: $f, count: ($f | length), error_hash: $h, tamper: $t}')"
printf '%s\n' "$json" | tee "$loop/verify-run.json"
