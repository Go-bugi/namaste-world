# bats 공통 준비. 테스트마다 임시 git 저장소를 만들고 autopilot 스크립트와 픽스처를 넣는다.

SRC_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
SCRIPTS="$SRC_ROOT/.specify-src/extensions/autopilot/scripts"
FIXTURES="$BATS_TEST_DIRNAME/fixtures"

speckit_python() {
  if [[ -n "${SPECKIT_PYTHON_EXECUTABLE:-}" ]]; then
    echo "$SPECKIT_PYTHON_EXECUTABLE"
  else
    echo "$(uv tool dir)/specify-cli/bin/python"
  fi
}

# 임시 저장소를 만들고 그 안으로 이동한다. memory에 넣을 Source 줄을 인자로 받는다.
make_repo() {
  local source_ref="${1:-}"
  REPO="$BATS_TEST_TMPDIR/repo"
  mkdir -p "$REPO"
  cd "$REPO"
  git init -q -b feature/test
  git config user.email t@example.com
  git config user.name test
  mkdir -p .specify/memory .specify/extensions/autopilot src-tauri .fixture-bin
  cp -R "$SCRIPTS" .specify/extensions/autopilot/scripts
  cp "$FIXTURES/agent.sh" .fixture-bin/agent.sh
  echo "# Constitution" >.specify/memory/constitution.md
  printf '# Main Project Specification\n\n## Requirements\n\n### Functional Requirements\n\n%s\n' "$source_ref" >.specify/memory/spec.md
  printf '# Changelog\n\n## Merged Features Log\n\n## Unresolved Contradictions\n' >.specify/memory/changelog.md
  printf '.specify/feature.json\n.specify/autopilot/\n.specify/workflows/runs/\n.fixture/\n' >.gitignore
  git add -A
  git commit -qm init
  export PATH="$FIXTURES/bin:$PATH"
}

# 기능 디렉터리를 직접 만든다(스크립트 단위 테스트용).
make_feature() {
  F=specs/001-demo
  mkdir -p "$F/loop" "$F/clarify" "$F/checklists" "$F/contracts"
  printf '{"feature_directory": "%s"}\n' "$F" >.specify/feature.json
  printf '## Requirements\n\n- **FR-DEMO-001**: x\n- **FR-DEMO-002**: y\n' >"$F/spec.md"
}

# 실제 workflow.yml을 셸 전용으로 바꿔 시나리오를 실행한다. 결과 상태를 STATUS에 담는다.
run_loop() {
  local scenario="$1"
  shift
  "$(speckit_python)" -I "$FIXTURES/to-shell-workflow.py" \
    "$SRC_ROOT/.specify-src/workflows/namaste/workflow.yml" "$BATS_TEST_TMPDIR/wf.yml"
  SCENARIO="$scenario" run specify workflow run "$BATS_TEST_TMPDIR/wf.yml" \
    --input spec=demo --input clarify_verdict=answered "$@"
  STATUS="$(grep -oE 'Status: [a-z]+' <<<"$output" | tail -1 | cut -d' ' -f2)"
}

calls() { local n; n="$(grep -cx "$1" .fixture/calls 2>/dev/null || true)"; echo "${n:-0}"; }

halt_reason() { grep -m1 '^- 이유:' specs/001-demo/loop/halt.md 2>/dev/null || echo "(halt.md 없음)"; }
