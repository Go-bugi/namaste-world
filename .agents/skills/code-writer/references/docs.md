# 공식 문서 확인

코드를 쓰다가 API나 동작이 확실하지 않을 때 찾아보는 순서와 문서 목록입니다.

## 찾아봐야 할 때

- API 이름, 인수, 반환 타입이 확실하지 않을 때
- 버전마다 동작이 다를 수 있을 때. Tauri v1과 v2, React 19의 새 API, zustand v4와 v5가 자주 헷갈립니다
- 스킬 예시가 설치된 버전과 맞는지 의심될 때
- 에러 메시지의 원인을 모를 때

## 찾는 순서

1. **설치된 버전을 확인합니다.** `package.json`과 `src-tauri/Cargo.toml`(정확한 버전은 `pnpm-lock.yaml`, `Cargo.lock`)을 읽습니다. 버전은 이 문서에 적지 않고 매번 확인합니다.
2. **설치된 타입 정의를 봅니다.** `node_modules/<패키지>/`의 `.d.ts`, Rust 크레이트는 docs.rs의 같은 버전 페이지입니다. 설치된 버전의 정확한 API는 여기가 가장 빠릅니다.
3. **공식 문서를 봅니다.** 아래 표의 주소입니다. 버전별 문서가 따로 있으면 설치된 버전의 문서를 엽니다.
4. **공식 저장소의 릴리스 노트와 이슈를 봅니다.** 버그나 변경 이력을 확인할 때만 봅니다.

블로그, Q&A 글, AI가 만든 요약만 근거로 쓰지 않습니다. 참고는 할 수 있지만 위 순서로 다시 확인합니다.

## 문서 목록

| 대상 | 문서 |
| --- | --- |
| React | https://react.dev/reference/react |
| Rust 표준 라이브러리, 언어 | https://doc.rust-lang.org/std/ , https://doc.rust-lang.org/book/ |
| Rust 크레이트(serde, thiserror, tokio 등) | `https://docs.rs/<크레이트>/<버전>` |
| Tauri | https://v2.tauri.app/ |
| Tauri JS API | https://v2.tauri.app/reference/javascript/api/ |
| Tauri Rust API | https://docs.rs/tauri/2 |
| Tauri 권한(capabilities) | https://v2.tauri.app/security/capabilities/ |
| neverthrow | https://github.com/supermacro/neverthrow (README가 공식 문서) |
| zustand | https://zustand.docs.pmnd.rs/ |
| React Three Fiber | https://r3f.docs.pmnd.rs/ |
| drei | https://drei.docs.pmnd.rs/ |
| three.js | https://threejs.org/docs/ |
| immer | https://immerjs.github.io/immer/ |
| Vitest | https://vitest.dev/ |
| fast-check | https://fast-check.dev/ |
| fast-check와 Vitest 연동(`@fast-check/vitest`) | https://fast-check.dev/docs/ecosystem/ |
| ts-rs | https://docs.rs/ts-rs/12 (설치된 버전에 맞춰 버전을 바꿉니다) |
| Vite | https://vite.dev/config/ |
| leva | https://github.com/pmndrs/leva (README가 공식 문서) |

- **Tauri는 v2 문서만 봅니다.** 검색하면 v1 문서(`tauri.app/v1`)가 자주 섞여 나옵니다. v1과 v2는 권한 모델과 플러그인 API가 다릅니다.
- **ts-rs는 버전마다 설정이 다릅니다.** `TS_RS_LARGE_INT` 같은 환경 변수는 12부터 있습니다. 낮은 버전에서는 조용히 무시됩니다.
- **three.js는 버전마다 API가 자주 바뀝니다.** 예시 코드가 설치된 버전에 있는 API인지 타입 정의(`@types/three`)로 확인합니다.

## 찾은 결과 쓰기

- 보고에 참고한 문서 주소를 적습니다.
- 코드 주석에는 이유만 씁니다. 버그를 피하는 코드처럼 문서를 봐야 이해되는 경우에만 그 문서나 이슈 주소를 한 줄 남깁니다.
