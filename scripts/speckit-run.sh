#!/usr/bin/env bash
# spec-kit autopilot 루프를 실행한다. 자세한 사용법은 README의 "spec-kit으로 기능 만들기".
#
#   scripts/speckit-run.sh --spec "무엇을 왜" [--plan "기술 방향"] [--maker claude|codex] [--checker claude|codex]
#   scripts/speckit-run.sh --spec-file design.md [--plan-file tech.md]
#   scripts/speckit-run.sh --resume <run_id> [--gate1 approve] ...
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# macOS가 잠자기에 들어가면 에이전트의 네트워크가 끊기고 단계 timeout 시계도 멈춘다. 실행하는 동안 잠자기를 막는다.
if [[ "$(uname)" == Darwin && -z "${SPECKIT_CAFFEINATED:-}" ]] && command -v caffeinate >/dev/null 2>&1; then
  SPECKIT_CAFFEINATED=1 exec caffeinate -i "$PWD/scripts/speckit-run.sh" "$@"
fi

usage() { sed -n '2,7p' "$0" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }

spec="" plan="" maker="claude" checker="" resume=""
verdicts=()
while (($#)); do
  case "$1" in
    --spec) spec="$2"; shift 2 ;;
    --spec-file) [[ -f "$2" ]] || { echo "파일이 없습니다: $2" >&2; exit 1; }; spec="$2"; shift 2 ;;
    --plan) plan="$2"; shift 2 ;;
    --plan-file) [[ -f "$2" ]] || { echo "파일이 없습니다: $2" >&2; exit 1; }; plan="$2"; shift 2 ;;
    --maker) maker="$2"; shift 2 ;;
    --checker) checker="$2"; shift 2 ;;
    --resume) resume="$2"; shift 2 ;;
    --clarify) verdicts+=(--input "clarify_verdict=$2"); shift 2 ;;
    --gate1) verdicts+=(--input "gate1_verdict=$2"); shift 2 ;;
    --gate2) verdicts+=(--input "gate2_verdict=$2"); shift 2 ;;
    -h | --help) usage 0 ;;
    *) echo "알 수 없는 옵션: $1" >&2; usage 1 ;;
  esac
done

case "$maker" in claude | codex) ;; *) echo "--maker는 claude나 codex입니다" >&2; exit 1 ;; esac
if [[ -z "$checker" ]]; then
  # checker는 maker가 아닌 쪽. 그 CLI가 없으면 maker가 맡는다.
  other=$([[ "$maker" == claude ]] && echo codex || echo claude)
  if command -v "$other" >/dev/null 2>&1; then checker="$other"; else checker="$maker"; fi
fi

# spec-kit 스크립트가 preset 템플릿을 고를 때 PyYAML이 있는 파이썬이 필요하다.
export SPECKIT_PYTHON_EXECUTABLE="${SPECKIT_PYTHON_EXECUTABLE:-$(uv tool dir)/specify-cli/bin/python}"
# 에이전트 권한: Claude는 파일 수정을 자동 승인하고 셸 명령은 .claude/settings.json 허용 목록만 쓴다.
# Codex는 작업 디렉터리 안에서만 쓰기를 허용하는 sandbox로 돈다. 권한 확인을 통째로 끄는 옵션은 쓰지 않는다.
# 루프 안의 에이전트는 커밋, push, 브랜치 이동, 되돌리기를 하지 않는다(사람이 끝에 커밋한다).
export SPECKIT_INTEGRATION_CLAUDE_EXTRA_ARGS="${SPECKIT_INTEGRATION_CLAUDE_EXTRA_ARGS:---permission-mode acceptEdits --disallowedTools 'Bash(git commit:*)' 'Bash(git push:*)' 'Bash(git reset:*)' 'Bash(git checkout:*)' 'Bash(git switch:*)' 'Bash(rm -rf:*)'}"
export SPECKIT_INTEGRATION_CODEX_EXTRA_ARGS="${SPECKIT_INTEGRATION_CODEX_EXTRA_ARGS:---sandbox workspace-write}"

if [[ -n "$resume" ]]; then
  exec specify workflow resume "$resume" "${verdicts[@]+"${verdicts[@]}"}"
fi

[[ -n "$spec" ]] || { echo "--spec 또는 --spec-file이 필요합니다" >&2; usage 1; }

exec specify workflow run namaste \
  --input "spec=$spec" \
  --input "plan=$plan" \
  --input "maker=$maker" \
  --input "checker=$checker" \
  "${verdicts[@]+"${verdicts[@]}"}"
