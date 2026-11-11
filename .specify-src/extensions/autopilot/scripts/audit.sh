#!/usr/bin/env bash
# C2: archive 뒤에 기능 spec의 FR ID가 memory에 빠짐없이 들어갔는지 대조한다.
# 각 ID는 .specify/memory/spec.md의 [Source: <기능>/spec.md -> ID]로 있거나,
# .specify/memory/changelog.md에 그 기능과 ID가 함께 적혀 있어야 한다(대체, 보류, 미해결 모순).
source "$(dirname "$0")/lib.sh"

dir="$(feature_dir)"
loop="$(loop_dir)"
memory_spec=".specify/memory/spec.md"
changelog=".specify/memory/changelog.md"

ids="$(grep -oE '\*\*FR-[A-Z0-9]+(-[A-Z0-9]+)*\*\*' "$dir/spec.md" | tr -d '*' | sort -u)"
missing=()
total=0
while IFS= read -r id; do
  [[ -z "$id" ]] && continue
  total=$((total + 1))
  ref="$dir/spec.md -> $id"
  grep -qF "$ref" "$memory_spec" 2>/dev/null && continue
  grep -F "$dir" "$changelog" 2>/dev/null | grep -qw -- "$id" && continue
  missing+=("$id")
done <<<"$ids"

if ((${#missing[@]})); then missing_json="$(printf '%s\n' "${missing[@]}" | jq -R . | jq -sc .)"; else missing_json='[]'; fi
json="$(jq -n --argjson t "$total" --argjson m "$missing_json" \
  '{total: $t, missing: $m, pass: (($m | length) == 0 and $t > 0), again: (($m | length) > 0 or $t == 0)}')"
printf '%s\n' "$json" | tee "$loop/audit.json"
