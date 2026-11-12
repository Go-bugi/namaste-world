# 테스트 질문

test-writer가 스펙과 설계만으로 정하지 못해 테스트하지 않은 동작입니다.

1. **Rust 속성 테스트 도구**: 규칙 문서(`fp-apply`의 `references/rust.md`)는 성질 검증에 `proptest`를 권하지만 `src-tauri/Cargo.toml`의 `[dev-dependencies]`에 없습니다. 그래서 `search_dirs`의 "중복 없음, PATH 순서 유지"와 `parse_distro_list`의 "도구용 배포판은 어떤 입력에서도 빠짐"을 예시 테스트로만 썼습니다. `proptest`를 추가할까요?
2. **Windows 기본 폴더의 경로 표기**: `search_dirs(HostOs::Windows, ...)` 테스트는 기본 폴더를 `home.join(".local").join("bin")`처럼 `Path::join`으로 만든 값과 비교합니다. CI(Linux)에서는 `C:\Users\u/.local/bin`이 됩니다. 구현이 `\`를 직접 붙여 문자열로 만들면 이 테스트와 다릅니다. `Path::join`으로 만든다고 보면 될까요?
3. **`parse_version`의 토큰 경계**: "첫 `숫자.숫자.숫자` 토큰"에서 `v2.1.322`, `2.1.322-beta.1`, `2.1`처럼 앞뒤에 글자가 붙거나 자리가 모자란 경우의 결과가 정해져 있지 않아 테스트하지 않았습니다.
4. **`parse_probe_output`에서 같은 에이전트가 두 줄 나올 때**: 첫 줄과 마지막 줄 중 어느 것을 쓰는지 정해져 있지 않아 테스트하지 않았습니다.
5. **`parse_marked_path`에서 마커가 세 번 이상 나올 때**: 어느 구간을 PATH로 볼지 정해져 있지 않아 테스트하지 않았습니다.
6. **`AgentDetectionPanel` 컴포넌트 테스트**: `package.json`에 React 컴포넌트를 렌더링해 볼 도구(`@testing-library/react`, jsdom 등)가 없고 Vitest 환경이 `node`라서, FR-AGENT-008의 화면 동작은 `detectionView.ts`(`toRows`, `panelReducer`, `canRedetect`)로만 검증했습니다. 컴포넌트 테스트가 필요하면 도구 추가가 먼저입니다.
7. **명령과 보관 상태(FR-AGENT-007, FR-AGENT-006)**: `agent_detection`, `redetect_agents`, `state.rs`의 `watch` 보관 동작은 설계에 시그니처만 있고 Tauri `State`를 거쳐야 해서, Tauri 없이 호출할 수 있는 함수(예: `watch::Sender`를 받는 함수)가 설계에 없습니다. 보관한 결과 요청, 시작 감지 대기, 다시 감지 중 이전 결과 반환을 테스트하려면 Tauri와 무관한 시그니처가 필요합니다. 추가할까요?

## 답 (2026-11-12)

1. `proptest`를 `[dev-dependencies]`에 추가합니다. 성질 테스트를 더해 주세요.
2. 맞습니다. Windows 기본 폴더도 `Path::join`으로 만듭니다.
3. 첫 줄에서 공백으로 나눈 토큰 가운데 (앞의 `v` 하나를 뗀 뒤) `숫자.숫자.숫자`로 시작하는 첫 토큰 전체를 씁니다. `v2.1.322` → `2.1.322`, `2.1.322-beta.1` → `2.1.322-beta.1`, `2.1` → `None`.
4. 첫 줄을 씁니다(PATH에서 먼저 찾은 것).
5. 처음 두 마커 사이를 씁니다.
6. 컴포넌트 테스트 도구는 이번에 넣지 않습니다. 화면은 `detectionView.ts` 테스트와 직접 확인으로 검증합니다.
7. `state.rs`의 `AgentDetection`(`new`, `latest`, `publish`)을 plan.md 시그니처에 추가했습니다. Tauri 없이 부를 수 있으니 FR-AGENT-007을 테스트해 주세요. FR-AGENT-006은 직접 확인합니다.
