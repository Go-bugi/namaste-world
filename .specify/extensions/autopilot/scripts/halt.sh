#!/usr/bin/env bash
# 루프를 멈추고 이유를 loop/halt.md에 남긴 뒤 실패로 끝낸다.
#   halt.sh <이유> <사람이 할 일>
source "$(dirname "$0")/lib.sh"

reason="${1:?이유가 필요합니다}"
todo="${2:-loop/ 아래 기록을 보고 원인을 고친 뒤 새로 실행합니다.}"

if dir="$(feature_dir 2>/dev/null)"; then
  out="$dir/loop/halt.md"
  mkdir -p "$dir/loop"
else
  mkdir -p "$AUTOPILOT_SCRATCH"
  out="$AUTOPILOT_SCRATCH/halt.md"
fi

{
  echo "# autopilot HALT"
  echo
  echo "- 시각: $(date '+%Y-%m-%d %H:%M:%S')"
  echo "- 브랜치: $(git branch --show-current)"
  echo "- 이유: $reason"
  echo
  echo "## 사람이 할 일"
  echo
  echo "$todo"
  echo
  echo "상한을 늘려 이어 가지 않습니다. 원인을 고친 뒤 \`scripts/speckit-run.sh\`로 새로 실행합니다."
  if [[ -n "${dir:-}" && -d "$dir/loop" ]]; then
    echo
    echo "## 마지막 검사 결과"
    for f in "$dir"/loop/*.json; do
      [[ -f "$f" ]] || continue
      echo
      echo "### $(basename "$f")"
      echo
      echo '```json'
      cat "$f"
      echo '```'
    done
  fi
} >"$out"

printf 'HALT: %s\n자세한 내용: %s\n' "$reason" "$out" >&2
exit 1
