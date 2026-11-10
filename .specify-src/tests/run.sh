#!/usr/bin/env bash
# pnpm speckit:check: autopilot 스크립트 정적 검사, workflow 정의 로드, bats 테스트.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

echo "==> shellcheck"
shellcheck -x -P .specify-src/extensions/autopilot/scripts \
  .specify-src/extensions/autopilot/scripts/*.sh \
  scripts/speckit-setup.sh scripts/speckit-run.sh \
  .specify-src/tests/fixtures/agent.sh .specify-src/tests/run.sh

echo "==> workflow 정의 로드"
specify workflow info .specify-src/workflows/namaste/workflow.yml >/dev/null

echo "==> 설치본과 원본 비교"
diff -rq -x .specify-dev .specify-src/extensions/autopilot .specify/extensions/autopilot
diff -rq -x .composed .specify-src/presets/namaste .specify/presets/namaste
cmp -s .specify-src/workflows/namaste/workflow.yml .specify/workflows/namaste/workflow.yml

echo "==> bats"
bats .specify-src/tests
