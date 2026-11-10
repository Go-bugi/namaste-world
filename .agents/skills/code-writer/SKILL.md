---
name: code-writer
description: 이 저장소에서 파일을 바꾸는 코드 작업이면 다른 코드 스킬(fp-apply, zustand-state-management, vercel-react-best-practices, threejs-*)보다 먼저 부르는 지휘 스킬입니다. 작업을 층(core, 상태, React, 3D, IO, 빌드와 앱 설정)으로 나누고 필요한 스킬만 골라 정해진 단계에 맞춰 적용하며, 모르는 API는 공식 문서로 확인합니다. 요청을 처리하면 `src/renderer`, `src-tauri`, 빌드와 앱 설정(`vite.config.ts`, `tsconfig*.json`, `package.json`, `Cargo.toml`, `tauri.conf.json`, capability 파일) 중 하나라도 바뀐다면, 문장에 "코드"라는 말이 없어도 반드시 이 스킬부터 사용하세요. 예: "기능 만들어 줘", "구현해 줘", "컴포넌트 추가해 줘", "Hook 만들어 줘", "store 만들어 줘", "씬에 ~ 넣어 줘", "큐브가 마우스 따라오게 해 줘", "useFrame 고쳐 줘", "GLTF 모델 로드해 줘", "Rust 명령 추가해 줘", "창 추가해 줘", "테스트 써 줘", "이 버그 고쳐 줘", "타입 에러 나는데 봐 줘", "버튼이 안 눌려", "리렌더 줄여 줘", "느린 거 고쳐 줘", "스타일 바꿔 줘", "alias 추가해 줘", "순수 함수로 바꿔 줘", "리팩터링해 줘". 질문이나 리뷰로 시작했더라도 파일을 고치기로 한 시점에 이 스킬을 부르고, 리뷰와 수정을 함께 요청받으면 이 스킬로 시작합니다. 파일을 바꾸지 않는 리뷰와 설명, 커밋, PR, 문서만 쓰는 작업은 이 스킬의 범위가 아닙니다.
---

# code-writer

코드를 쓸 때 어떤 스킬을 어떤 순서로 부를지 정하는 스킬입니다. 규칙은 직접 담지 않습니다. 저장소 규칙은 `AGENTS.md`, 설계 원칙은 `fp-apply`, 라이브러리 사용법은 각 스킬에 있고 이 문서는 그 위치만 가리킵니다. 같은 규칙을 두 곳에 적으면 언젠가 어긋나기 때문입니다.

## 절차

| 단계 | 하는 일 | 근거 |
| --- | --- | --- |
| 0. 맥락 | `AGENTS.md`의 "구조", "폴더 구조"를 읽습니다 | `AGENTS.md` |
| 1. 분류 | 이번 작업이 건드리는 층을 고릅니다. core, 상태(store), React, 3D, IO(TS `api/`, Rust 명령과 IO 모듈), 빌드와 앱 설정(창, 권한, CSP, alias, 의존성) | 아래 라우팅 표 |
| 2. 모델링 | 타입, 상태 전이, 에러 유니온을 정하고 core 함수의 시그니처를 먼저 씁니다 | `fp-apply` |
| 3. 테스트 | `AGENTS.md` "명령어"의 테스트 규칙에 따라 테스트를 먼저 쓸지 정합니다. 테스트만 요청받았으면 이 단계가 본 작업이고 2, 4, 5단계는 건너뜁니다 | `AGENTS.md`, `fp-apply` 테스트 절 |
| 4. core 구현 | 순수 함수를 `AGENTS.md` "폴더 구조"가 정한 위치에 씁니다 | `fp-apply` |
| 5. shell 연결 | store, 컴포넌트, `useFrame`, `api/`, Rust 명령을 층마다 해당 스킬로 씁니다. shell은 core를 부르기만 합니다 | 라우팅 표 |
| 6. 검증 | 아래 "완료 기준"을 확인합니다 | |
| 7. 리뷰 | 아래 "리뷰"를 따릅니다 | |
| 8. 보고 | 무엇을 어디에 두었는지, 검증 결과, 참고한 문서, 남은 결정을 알립니다 | |

2~4단계를 끝낸 뒤에 5단계로 갑니다. 그래야 shell에 계산이 섞이지 않습니다.

**작은 수정**은 바뀌는 파일이 1~2개이고 core와 IO를 건드리지 않는 작업입니다(스타일, 문구, 설정 한 줄 등). 이때는 0, 1, 5, 6, 8단계만 하고, 6단계에서는 바뀐 언어의 검사만 돌립니다.

## 라우팅

건드리는 층에 해당하는 것만 불러옵니다. threejs 스킬은 필요한 것만 하나씩 엽니다. `fp-apply`가 나오는 행은 그 스킬의 SKILL.md를 먼저 읽고 TS 작업이면 `references/typescript.md`도 함께 읽습니다. `fp-apply`의 문서 경로는 그 스킬 폴더 기준입니다.

| 작업 | 불러올 것 |
| --- | --- |
| 계산, 검증, 상태 전이, 에러 처리 | `fp-apply`(`references/typescript.md` 또는 `references/rust.md`) |
| 테스트 | `fp-apply`의 테스트 절과 언어별 문서의 "테스트", `AGENTS.md` "명령어" |
| zustand store | `zustand-state-management`, `fp-apply`의 `references/zustand.md` |
| React 컴포넌트, Hook | `vercel-react-best-practices`, `fp-apply`의 `references/react.md` |
| CSS, 스타일 | `AGENTS.md` "구조"의 웹뷰 항목 |
| Canvas 안, `useFrame` | `threejs-react-three-fiber`, `fp-apply`의 `references/r3f.md`, `AGENTS.md` "스킬"의 R3F와 zustand 항목 |
| 씬 구성, 카메라, 렌더러 | `threejs-fundamentals` |
| 모델, 텍스처, HDR 로드 | `threejs-loaders`, `threejs-asset-pipeline`, `threejs-textures`, `AGENTS.md` "스킬"의 3D CSP 제약 |
| 애니메이션 | `threejs-animation` |
| 재질, 조명, 셰이더, 후처리 | `threejs-materials`, `threejs-lighting`, `threejs-shaders`, `threejs-postprocessing` |
| 클릭, 드래그, 카메라 조작 | `threejs-interaction` |
| 지오메트리, 인스턴싱 | `threejs-geometry` |
| React 리렌더, 번들 크기 | `vercel-react-best-practices` |
| 3D 프레임 저하, draw call | `threejs-performance` |
| TS `api/`, `types/` | `fp-apply`의 `references/typescript.md`("사이드 이펙트와 비동기"), `AGENTS.md` "구조"의 기능 추가 단계와 "Rust와 TS 타입 맞추기" |
| Rust 명령, 이벤트, 백그라운드 작업 | `fp-apply`의 `references/rust.md`, `AGENTS.md` "구조"의 기능 추가 단계, "폴더 구조", "Rust와 TS 타입 맞추기" |
| 권한, CSP | `AGENTS.md` "구조"의 기능 추가 3단계와 CSP 항목 |
| 빌드 설정, alias, 의존성 | `AGENTS.md` "구조"의 tsconfig 항목과 의존성 항목 |
| 창 추가 | `AGENTS.md` "창 추가하기" |
| 선언에 붙는 문서 주석, 여러 줄 주석 | `korean-tech-writing`, `AGENTS.md` "문서와 주석" |
| 한 줄 주석 | `AGENTS.md` "문서와 주석" |

느린 원인을 모르면 먼저 측정해 React 쪽인지 3D 쪽인지 정한 뒤 해당 행을 따릅니다.

## 스킬끼리 다를 때

`AGENTS.md`의 "스킬끼리 다를 때"를 따릅니다(`AGENTS.md` → `fp-apply` → 외부 스킬). 외부 스킬은 원본 그대로라 이 앱과 맞지 않는 내용이 있습니다. 목록에 없는 충돌을 찾으면 코드에서 조용히 우회하지 말고 보고에 적어 `AGENTS.md`에 추가할지 묻습니다.

## 모르는 것은 공식 문서로

API 이름이나 시그니처가 확실하지 않거나 버전마다 동작이 다를 수 있으면 추측으로 쓰지 않고 확인합니다. 찾는 순서와 문서 목록은 [references/docs.md](references/docs.md)에 있습니다.

- 스킬 내용과 설치된 버전의 공식 문서가 다르면 공식 문서를 따르고 어긋난 점을 보고에 적습니다.
- 공식 문서와 `AGENTS.md`가 다르면 고치기 전에 사용자에게 묻습니다.
- 확인해도 확실하지 않으면 그 사실을 보고에 적습니다.

## 리뷰

바뀐 파일이 5개 이상이거나 TS와 Rust를 함께 바꿨으면 서브에이전트에게 리뷰를 맡깁니다. 바뀐 파일 목록과 `fp-apply`의 리뷰 체크리스트, 건드린 층의 언어별 체크리스트를 넘깁니다. 쓴 사람이 놓친 것을 잡기 위한 단계라서 리뷰어에게 작성 과정은 넘기지 않습니다.

변경이 그보다 작거나 서브에이전트를 쓸 수 없는 환경이면 같은 체크리스트를 직접 확인합니다. 그렇게 했다는 사실도 보고에 적습니다.

## 완료 기준

`AGENTS.md` "명령어"에 적힌 PR CI 검사를 로컬에서 돌립니다. 순서는 이렇습니다.

1. Rust를 건드렸으면 Rust 검사를 먼저 돌립니다. `pnpm test:rust`가 TS 타입(`types/bindings/`)을 다시 만들기 때문입니다. 바뀐 생성 파일이 있으면 함께 커밋 대상에 넣습니다.
2. 그다음 TS 검사를 돌립니다. `pnpm build`가 typecheck도 함께 돌립니다.

- 검사가 모두 통과합니다. 실패하면 고치고, 고칠 수 없으면 출력과 함께 보고합니다.
- `fp-apply` 리뷰 체크리스트와 건드린 층의 언어별 체크리스트를 확인했습니다.
- core가 `AGENTS.md`가 정한 위치에 있고, 금지된 import가 없습니다.
- 건너뛴 단계나 확인하지 못한 항목이 있으면 그대로 보고합니다.
