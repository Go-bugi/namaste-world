# Namaste World Constitution

spec-kit 명령(`specify`, `plan`, `tasks`, `analyze`, `implement`, `converge`)과 autopilot 루프가 따르는 프로젝트 원칙입니다. `analyze`는 이 문서를 어긴 항목을 CRITICAL로 매깁니다. 원칙을 바꾸려면 아래 Governance의 절차를 따릅니다.

## Core Principles

### I. 코드 규칙은 저장소 스킬을 따른다

코드를 쓰고 고치는 단계는 `AGENTS.md`, `code-writer` 스킬, `fp-apply` 스킬을 따릅니다. 이 문서는 그 규칙을 옮겨 적지 않습니다. 같은 규칙을 두 곳에 적으면 언젠가 어긋나기 때문입니다.

- 절차(맥락, 층 분류, 모델링, 테스트, core 구현, shell 연결, 검증, 리뷰, 보고)는 `code-writer`가 정합니다.
- 계산과 사이드 이펙트를 나누는 기준, 테스트 방식, 리뷰 체크리스트는 `fp-apply`에 있습니다.
- 폴더 위치(`domain/`, `api/`, `commands/` 등), Tauri 경계, CSP, 빌드 타깃 제약은 `AGENTS.md`에 있습니다.
- 스킬끼리 다르면 `AGENTS.md`, `fp-apply`, 외부 스킬 순서로 따릅니다.

`plan`과 `analyze`는 이 원칙을 검사할 때 위 문서를 직접 읽습니다. plan이 core 위치를 `AGENTS.md` "폴더 구조"와 다르게 잡거나, 계산을 shell(컴포넌트, store, `useFrame`, Rust 명령)에 두면 이 원칙 위반입니다.

### II. 테스트는 구현보다 먼저, 구현을 모르는 쪽이 쓴다 (NON-NEGOTIABLE)

- core(순수 함수)를 새로 만들거나 바꾸는 기능은 테스트를 먼저 씁니다. 테스트 위치와 러너는 `AGENTS.md` "명령어"를 따릅니다.
- autopilot 루프에서는 `test-writer` 서브에이전트가 spec과 contract만 보고 테스트를 씁니다. 구현하는 에이전트는 테스트 파일을 고치지 않습니다.
- 테스트가 틀렸다고 판단하면 고치지 말고 `specs/<기능>/test-disputes.md`에 근거를 적습니다.

### III. 기존 스펙을 읽고 시작한다

- 새 기능의 spec을 쓰기 전에 `.specify/memory/spec.md`와 `plan.md`에서 관련 도메인의 요구사항과 결정을 읽습니다.
- 기존 요구사항을 바꾸거나 없애는 기능은 spec에 그 사실과 대상 ID를 적습니다. 말없이 반대로 구현하지 않습니다.
- 기능이 머지되기 전에 `archive`로 기능 spec을 memory에 합칩니다. memory가 "구현된 기능의 스펙"의 기준입니다.

### IV. 범위는 spec이 정한다

- spec에 없는 동작, 옵션, 화면을 추가하지 않습니다. 필요하면 spec을 먼저 고칩니다.
- 사용자에게 보이는 연결(버튼, 메뉴, 라우트)은 `AGENTS.md` "브랜치 전략과 릴리스"의 규칙대로 마지막에 붙입니다.

### V. 검증은 CI와 같은 명령으로 한다

- 완료 기준은 `code-writer` "완료 기준"과 같습니다. `AGENTS.md` "명령어"의 PR CI 검사를 로컬에서 모두 통과해야 합니다.
- 검사를 통과시키려고 검사 설정을 끄거나 테스트를 지우지 않습니다.

## Spec Conventions

- **언어**: spec, plan, tasks의 본문은 한국어로 씁니다. `korean-tech-writing` 스킬의 문체를 따릅니다. 섹션 제목과 ID 접두사는 spec-kit 명령이 찾을 수 있게 템플릿의 영어 표기를 그대로 둡니다.
- **도메인 코드**: 영어 대문자 2~6자로 기능 영역을 나타냅니다. 아래 표에 없는 영역의 첫 기능을 만들 때 GATE-1에서 표에 추가합니다.

  | 코드 | 영역 |
  | --- | --- |
  | (아직 없음) | |

- **ID 형식**: 기능 요구사항은 `FR-<도메인>-NNN`(예: `FR-TRAY-001`), 성공 기준은 `SC-<도메인>-NNN`입니다. 기능 spec의 `**Domain**:` 필드에 주 도메인을 적습니다.
- **memory 형식**: `.specify/memory/spec.md`와 `plan.md`의 섹션 구조는 저장소에 커밋한 뼈대를 따릅니다. `[Source: ...]` 표기, `> **Revision**:` 줄, changelog 구조는 archive 확장이 정한 형식을 그대로 씁니다.

## Governance

- 이 문서는 다른 개발 관행보다 우선합니다. 단, 코드 규칙은 원칙 I에 따라 `AGENTS.md`와 스킬이 정합니다.
- 고칠 때는 `/speckit-constitution`(Codex는 `$speckit-constitution`)으로 고치고, 아래 버전을 올립니다. 원칙을 빼거나 뜻을 바꾸면 MAJOR, 원칙이나 제약을 추가하면 MINOR, 문장만 다듬으면 PATCH입니다.
- autopilot 루프 중에 원칙을 바꿔야 하면 GATE-1에서 사람이 결정합니다. 에이전트가 루프 안에서 이 문서를 고치지 않습니다.

**Version**: 1.0.0 | **Ratified**: 2026-11-10 | **Last Amended**: 2026-11-10
