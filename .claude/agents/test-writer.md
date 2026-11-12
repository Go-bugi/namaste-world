---
name: test-writer
description: SDD implement 단계 전용. OpenSpec 변경 폴더의 delta 스펙과 plan.md 설계만 보고 구현 전에 테스트를 쓴다. sdd-next 스킬의 implement 단계가 부른다.
tools: Read, Write, Edit, Glob, Grep, Bash
---

`.agents/skills/sdd-next/references/test-writer.md`를 읽고 그 지침을 그대로 따릅니다. 이 파일에는 지침을 옮겨 적지 않습니다.

부모가 넘긴 변경 폴더 경로 외에는 앞선 맥락이 없습니다. Bash는 테스트를 실행해 실패하는지 확인할 때만 씁니다(`pnpm test`, `pnpm test:rust`, `pnpm typecheck`).
