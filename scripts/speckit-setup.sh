#!/usr/bin/env bash
# spec-kit 도구를 고정 버전으로 설치하고 저장소 설정을 점검한다.
#
#   scripts/speckit-setup.sh            CLI 설치와 점검만 한다. 설정은 저장소에 커밋되어 있다.
#   scripts/speckit-setup.sh --rebuild  통합, preset, 확장을 고정 버전으로 다시 설치한다.
#                                       spec-kit이나 확장 버전을 올릴 때 쓴다.
set -euo pipefail

SPECKIT_VERSION="v1.2.3"
EXTENSIONS=(
  "archive|https://github.com/stn1slv/spec-kit-archive/archive/refs/tags/v1.4.0.zip"
  "reconcile|https://github.com/stn1slv/spec-kit-reconcile/archive/refs/tags/v1.3.1.zip"
  "memory-loader|https://github.com/KevinBrown5280/spec-kit-memory-loader/archive/refs/tags/v1.0.0.zip"
)
PRESET_DIR=".specify-src/presets/namaste"
AUTOPILOT_DIR=".specify-src/extensions/autopilot"

cd "$(git rev-parse --show-toplevel)"

log() { printf '\n==> %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

require() {
  command -v "$1" >/dev/null 2>&1 || fail "$1이(가) 필요합니다. $2"
}

install_cli() {
  require uv "https://docs.astral.sh/uv/ 에서 설치하세요."
  require git ""
  require jq "brew install jq 등으로 설치하세요."
  local want="${SPECKIT_VERSION#v}" have=""
  if command -v specify >/dev/null 2>&1; then
    have="$(specify version 2>/dev/null | grep -Eo 'CLI Version +[0-9.]+' | grep -Eo '[0-9.]+$' || true)"
  fi
  if [[ "$have" == "$want" ]]; then
    log "specify-cli $want 이미 설치됨"
  else
    log "specify-cli $want 설치 (현재: ${have:-없음})"
    uv tool install --force specify-cli --from "git+https://github.com/github/spec-kit.git@${SPECKIT_VERSION}"
  fi
}

speckit_python() {
  printf '%s/specify-cli/bin/python' "$(uv tool dir)"
}

# preset과 확장 명령은 "현재 기본 통합"의 스킬에만 쓰인다(spec-kit 1.2.3 동작).
# 그래서 codex를 기본으로 두고 한 번, claude로 되돌리고 한 번 등록한다.
render_for_both() {
  local kind="$1" id="$2"
  specify integration use codex >/dev/null
  specify "$kind" disable "$id" >/dev/null
  specify "$kind" enable "$id" >/dev/null
  specify integration use claude >/dev/null
}

rebuild() {
  log "spec-kit 초기화 (claude, codex)"
  specify init --here --force --non-interactive --integration claude --script sh >/dev/null
  specify integration install codex --script sh >/dev/null || specify integration upgrade codex --force >/dev/null

  log "namaste preset 설치"
  specify preset remove namaste >/dev/null 2>&1 || true
  specify integration use codex >/dev/null
  specify preset add --dev "$PRESET_DIR" >/dev/null
  specify integration use claude >/dev/null
  specify preset disable namaste >/dev/null
  specify preset enable namaste >/dev/null

  local entry id url
  for entry in "${EXTENSIONS[@]}"; do
    id="${entry%%|*}"
    url="${entry#*|}"
    log "확장 $id 설치 ($url)"
    specify extension remove "$id" --force >/dev/null 2>&1 || true
    echo y | specify extension add "$id" --from "$url" >/dev/null
    render_for_both extension "$id"
  done

  if [[ -d "$AUTOPILOT_DIR" ]]; then
    log "autopilot 확장 설치"
    specify extension remove autopilot --force >/dev/null 2>&1 || true
    specify extension add --dev "$AUTOPILOT_DIR" >/dev/null
    render_for_both extension autopilot
  fi

  # integration use가 core 스크립트의 실행 권한을 지운다(spec-kit 1.2.3).
  chmod +x .specify/scripts/bash/*.sh
}

check() {
  log "점검"
  local ok=1
  "$(speckit_python)" -c 'import yaml' 2>/dev/null || { echo "  ✗ specify-cli 파이썬에서 PyYAML을 불러올 수 없음"; ok=0; }
  local integ
  for integ in claude codex; do
    jq -e --arg i "$integ" '.installed_integrations | index($i)' .specify/integration.json >/dev/null \
      || { echo "  ✗ $integ 통합 없음"; ok=0; }
  done
  jq -e '.presets.namaste.enabled' .specify/presets/.registry >/dev/null 2>&1 || { echo "  ✗ namaste preset 없음"; ok=0; }
  local id
  for id in archive reconcile memory-loader; do
    jq -e --arg i "$id" '.extensions[$i].enabled' .specify/extensions/.registry >/dev/null 2>&1 \
      || { echo "  ✗ 확장 $id 없음"; ok=0; }
  done
  if [[ -d "$AUTOPILOT_DIR" ]]; then
    jq -e '.extensions.autopilot.enabled' .specify/extensions/.registry >/dev/null 2>&1 \
      || { echo "  ✗ 확장 autopilot 없음"; ok=0; }
  fi
  command -v claude >/dev/null 2>&1 || command -v codex >/dev/null 2>&1 || { echo "  ✗ claude나 codex CLI가 하나는 있어야 함"; ok=0; }
  [[ "$ok" == 1 ]] || fail "점검 실패"
  echo "  ✓ 설치와 설정이 모두 맞습니다"
  cat <<EOF

루프 없이 /speckit-* 명령을 직접 쓸 때는 셸 설정에 다음을 넣으세요.
spec-kit 스크립트가 템플릿을 고를 때 PyYAML이 있는 파이썬이 필요합니다.

  export SPECKIT_PYTHON_EXECUTABLE="$(speckit_python)"
EOF
}

install_cli
if [[ "${1:-}" == "--rebuild" ]]; then
  rebuild
elif [[ -n "${1:-}" ]]; then
  fail "알 수 없는 옵션: $1 (--rebuild만 받습니다)"
fi
check
