---
name: speckit-autopilot-review
description: 'autopilot B6: code-writer 7단계 리뷰. 바뀐 코드를 fp-apply 체크리스트로 보고 loop/review.md에 쓴다'
compatibility: Requires spec-kit project structure with .specify/ directory
metadata:
  author: namaste-world
  source: autopilot:commands/speckit.autopilot.review.md
---

## 입력

```text
$ARGUMENTS
```

## 하는 일

이 명령은 구현한 에이전트와 다른 에이전트가 실행합니다. 쓴 사람이 놓친 것을 잡는 단계라서 작성 과정은 보지 않습니다.

1. 리뷰 대상은 `git diff --name-only HEAD`와 추적되지 않은 새 파일 중 `src/`, `src-tauri/` 아래 코드입니다. 테스트 파일(`*.test.ts`, `*.test.tsx`)과 Rust 파일의 `#[cfg(test)] mod tests` 부분은 리뷰하지 않습니다(이 루프에서 테스트는 고정되어 있습니다).
2. 기준은 아래 문서뿐입니다. 이 문서들에 근거가 없는 취향 문제는 적지 않습니다.
   - `.agents/skills/fp-apply/SKILL.md`의 "리뷰 체크리스트"
   - 건드린 층의 언어별 문서 체크리스트(`.agents/skills/fp-apply/references/typescript.md`, `rust.md`, `react.md`, `zustand.md`, `r3f.md`의 "체크리스트")
   - `AGENTS.md`의 "구조", "폴더 구조"(위치, 금지된 import, Tauri 경계)
   - 현재 기능 디렉터리(`.specify/feature.json`의 `feature_directory`)의 `spec.md`와 `contracts/`(spec에 없는 동작, contract와 다른 시그니처)
3. 현재 기능 디렉터리의 `loop/review.md`를 새로 씁니다.

```markdown
# Review

- [파일:줄] <어긴 규칙과 근거 문서> / <고칠 방향 한 줄>

COUNTS: issues=<수>
```

지적이 없으면 목록 없이 `COUNTS: issues=0`만 씁니다. 마지막 줄은 반드시 `COUNTS:` 줄입니다.

## 하지 않는 일

- 코드를 고치지 않습니다. 고치는 일은 `speckit.autopilot.fix verify`가 합니다.