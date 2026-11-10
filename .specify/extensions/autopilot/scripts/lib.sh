#!/usr/bin/env bash
# autopilot 스크립트가 같이 쓰는 함수.
# 값을 계산하는 함수(이름이 count_, parse_, json_으로 시작)는 인자와 표준 입력만 읽고 표준 출력으로만 돌려준다.
# 파일을 쓰는 함수(write_, record_)는 따로 둔다.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

export AUTOPILOT_SCRATCH=".specify/autopilot"

die() { printf 'autopilot: %s\n' "$*" >&2; exit 2; }

# 현재 기능 디렉터리(저장소 기준 상대 경로). core specify가 .specify/feature.json에 기록한다.
feature_dir() {
  local fj=".specify/feature.json" dir
  [[ -f "$fj" ]] || die ".specify/feature.json이 없습니다. specify 단계가 끝났는지 확인하세요."
  dir="$(jq -r '.feature_directory // empty' "$fj")"
  [[ -n "$dir" ]] || die ".specify/feature.json에 feature_directory가 없습니다."
  dir="${dir#"$REPO_ROOT"/}"
  [[ -d "$dir" ]] || die "기능 디렉터리 ${dir}이(가) 없습니다."
  printf '%s' "$dir"
}

loop_dir() {
  local d
  d="$(feature_dir)/loop"
  mkdir -p "$d"
  printf '%s' "$d"
}

# 파일 하나에서 패턴에 맞는 줄 수. 파일이 없으면 0.
count_lines() {
  local pattern="$1" file="$2"
  [[ -f "$file" ]] || { echo 0; return; }
  grep -cE "$pattern" "$file" || true
}

# "COUNTS: a=1 b=2" 줄에서 key 값을 꺼낸다. 없으면 빈 문자열.
parse_count() {
  local key="$1" file="$2"
  [[ -f "$file" ]] || return 0
  grep -E '^COUNTS:' "$file" | tail -1 | grep -oE "${key}=[0-9]+" | cut -d= -f2 || true
}

sha() { shasum -a 256 | cut -c1-16; }

# 숫자와 시간 표기를 지운 오류 출력의 해시. 실행마다 바뀌는 값 때문에 같은 오류가 다르게 보이지 않게 한다.
error_signature() {
  sed -E 's/[0-9]+(\.[0-9]+)?(ms|s)\b//g; s/[0-9a-f]{7,}//g; s/[0-9]+//g' | sha
}

# 작업 트리 내용의 해시(커밋되지 않은 변경 포함). 진전 없음 판정에 쓴다.
# 루프가 매번 쓰는 기록(specs/*/loop/)은 빼야 "아무것도 안 바뀜"을 알아볼 수 있다.
worktree_hash() {
  local exclude=':(exclude,glob)specs/**/loop/**'
  {
    git diff HEAD -- . "$exclude"
    git ls-files --others --exclude-standard -z -- . "$exclude" | xargs -0 cat 2>/dev/null
  } | sha
}

# 이전 기록(jsonl 마지막 줄)의 값. 없으면 빈 문자열.
last_record() {
  local file="$1" key="$2"
  [[ -s "$file" ]] || return 0
  tail -1 "$file" | jq -r --arg k "$key" '.[$k] // empty'
}

record_history() {
  local file="$1" json="$2"
  printf '%s\n' "$json" >>"$file"
}

# 테스트로 고정할 범위의 해시. TS는 파일 전체, Rust는 #[cfg(test)] 줄부터 끝까지.
test_fingerprint() {
  local list="$1" f
  [[ -s "$list" ]] || { echo none; return; }
  while IFS= read -r f; do
    printf '== %s\n' "$f"
    if [[ ! -f "$f" ]]; then
      echo "(deleted)"
    elif [[ "$f" == *.rs ]]; then
      sed -n '/#\[cfg(test)\]/,$p' "$f"
    else
      cat "$f"
    fi
  done <"$list" | sha
}

# Rust 파일에서 테스트 모듈 앞부분(구현 쪽)의 해시.
rust_head_hash() {
  sed '/#\[cfg(test)\]/,$d' "$1" | sha
}
