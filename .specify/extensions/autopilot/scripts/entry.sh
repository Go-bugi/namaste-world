#!/usr/bin/env bash
# 루프 진입 조건. 하나라도 어긋나면 시작하지 않는다.
source "$(dirname "$0")/lib.sh"

branch="$(git branch --show-current)"
problems=()
[[ "$branch" == feature/* ]] || problems+=("현재 브랜치($branch)가 feature/*가 아닙니다")
[[ -z "$(git status --porcelain)" ]] || problems+=("커밋하지 않은 변경이 있습니다. 커밋하거나 stash한 뒤 실행합니다")
[[ -f .specify/memory/constitution.md ]] || problems+=(".specify/memory/constitution.md가 없습니다")
[[ -f .specify/memory/spec.md ]] || problems+=(".specify/memory/spec.md 뼈대가 없습니다")
command -v jq >/dev/null || problems+=("jq가 필요합니다")

if ((${#problems[@]})); then
  printf '진입 조건을 만족하지 않습니다.\n' >&2
  printf ' - %s\n' "${problems[@]}" >&2
  exit 1
fi

rm -rf "$AUTOPILOT_SCRATCH"
mkdir -p "$AUTOPILOT_SCRATCH"
jq -n --arg b "$branch" '{branch: $b, ok: true}'
