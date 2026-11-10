# Main Implementation Plan

구현되어 머지된 기능의 기술 결정을 하나로 합친 문서입니다. `/speckit-archive-run`이 `specs/<기능>/plan.md`의 내용을 아래 섹션에 합칩니다. 손으로 고치지 않습니다.

- 기능별 블록을 만들지 않고 공유 섹션 안에 합칩니다. 어느 기능에서 왔는지는 `[Source: ...]`로 남습니다.
- 코드 규칙과 폴더 구조는 `AGENTS.md`와 `code-writer`, `fp-apply` 스킬이 기준입니다. 이 문서에는 기능이 실제로 정한 결정만 남깁니다.

## Summary

## Technical Context

**Language/Version**: TypeScript(renderer), Rust stable(src-tauri)

**Primary Dependencies**: Tauri 2, React

**Storage**:

**Testing**: Vitest + fast-check(`pnpm test`), `cargo test`(`pnpm test:rust`)

**Target Platform**: macOS(WKWebView), Windows(WebView2)

**Project Type**: desktop-app

**Performance Goals**:

**Constraints**:

**Scale/Scope**:

## Layers & Skills

## Core Model

## Project Structure

### Source Code (repository root)
