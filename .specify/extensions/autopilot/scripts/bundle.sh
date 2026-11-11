#!/usr/bin/env bash
# B4-2 앞: test-writer 서브에이전트에게 줄 입력 묶음을 만들고, 범위 검사용 snapshot을 남긴다.
#   <기능>/loop/test-input/   spec 발췌, contracts, 시그니처 파일, 테스트 규칙 문서
#   <기능>/loop/pre-tests.txt "<내용 해시> <Rust 구현부 해시 또는 -> <경로>"
#   <기능>/loop/pre-snapshot/ 지금 바뀌어 있는 파일의 사본(범위를 벗어난 수정을 되돌릴 때 씀)
source "$(dirname "$0")/lib.sh"

dir="$(feature_dir)"
loop="$dir/loop"
input="$loop/test-input"
rm -rf "$input" "$loop/pre-snapshot"
mkdir -p "$input/contracts" "$input/signatures" "$input/rules" "$loop/pre-snapshot"

# spec 발췌: plan 쪽 내용 없이 요구사항, 시나리오, Clarifications만.
awk '
  /^## (User Scenarios|Requirements|Success Criteria|Clarifications)/ { keep = 1 }
  /^## (Assumptions)/ { keep = 0 }
  /^## / && !/^## (User Scenarios|Requirements|Success Criteria|Clarifications)/ { keep = 0 }
  keep
' "$dir/spec.md" >"$input/spec.md"

cp -R "$dir/contracts/." "$input/contracts/"

# 테스트 규칙 문서: code-writer 라우팅 표의 "테스트" 행이 가리키는 곳만.
extract_section() {
  local file="$1" start="$2"
  if [[ ! -f "$file" ]]; then
    echo "($file 없음. 이 규칙 없이 테스트를 씁니다.)"
    return
  fi
  awk -v s="$start" '
    !on && index($0, s) == 1 { on = 1; level = match($0, /[^#]/) - 1; print; next }
    on && /^#+ / { if (match($0, /[^#]/) - 1 <= level) exit }
    on { print }
  ' "$file"
}
extract_section .agents/skills/fp-apply/SKILL.md "### 11. 테스트" >"$input/rules/fp-apply-tests.md"
extract_section .agents/skills/fp-apply/references/typescript.md "## 테스트" >"$input/rules/typescript-tests.md"
extract_section .agents/skills/fp-apply/references/rust.md "## 테스트" >"$input/rules/rust-tests.md"
extract_section AGENTS.md "## 명령어" >"$input/rules/agents-commands.md"

: >"$loop/pre-tests.txt"
while IFS= read -r f; do
  [[ -z "$f" || ! -f "$f" ]] && continue
  case "$f" in "$loop"/*) continue ;; esac
  head="-"
  [[ "$f" == *.rs ]] && head="$(rust_head_hash "$f")"
  printf '%s %s %s\n' "$(sha <"$f")" "$head" "$f" >>"$loop/pre-tests.txt"
  mkdir -p "$loop/pre-snapshot/$(dirname "$f")"
  cp "$f" "$loop/pre-snapshot/$f"
  # B4-1이 만든 시그니처 파일(코드)만 묶음에 넣는다.
  case "$f" in
    src/*.ts | src/*.tsx | src-tauri/src/*.rs)
      mkdir -p "$input/signatures/$(dirname "$f")"
      cp "$f" "$input/signatures/$f"
      ;;
  esac
done <<<"$({ git diff --name-only HEAD; git ls-files --others --exclude-standard; } | sort -u)"

jq -n --arg d "$input" --argjson n "$(find "$input/signatures" -type f | wc -l)" '{input: $d, signature_files: $n}'
