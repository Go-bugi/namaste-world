# Test questions

### TQ1. `shell::wsl::login_shell_script`를 무엇으로 검증할까요?

- 근거: FR-AGENT-013, plan.md `## 설계` > 시그니처 `wsl.rs`

이 함수는 문자열을 만들 뿐이고, 로그인 셸 선택(bash/zsh면 `-ilc`, 그 밖이나 못 찾으면 `/bin/sh -lc`)과 `inner` 인용이 맞는지는 스크립트를 셸에서 돌려 봐야 알 수 있습니다. 아래 중 어느 쪽으로 테스트할지 정해 주세요. 지금은 `after_fence`만 테스트했습니다.

1. 문자열 구성만 검사한다(`getent passwd`, `-ilc`, `/bin/sh -lc`가 들어 있음, `inner`의 작은따옴표가 깨지지 않음 등). 이 경우 정확히 어떤 조각을 고정할지도 알려 주세요.
2. 테스트에서 `/bin/sh`로 실제 실행해 본다(`getent` 대신 쓸 가짜를 어떻게 넣을지 함께 정해야 함).
3. 단위 테스트를 두지 않고 실제 배포판에서 수동으로 확인한다.

**답**: 문자열 검사와 실제 실행을 함께 함. 문자열에 `getent passwd`, bash/zsh용 `-ilc`, 대체 경로 `/bin/sh -lc`가 들어 있는지 확인함. 또 macOS 테스트에서 `/bin/sh -c`로 실제 실행함. macOS에는 `getent`가 없어 대체 경로를 타는데, 이때 경계 마커가 찍히고 작은따옴표가 든 `inner`도 그대로 실행되는지 확인함

### TQ2. 셸 환경 읽기의 실패 분류, 시간 상한, 환경 재사용을 테스트할 순수 함수나 trait이 필요합니다

- 근거: FR-AGENT-015, FR-AGENT-016, FR-AGENT-018, SC-AGENT-003, plan.md `## 설계` > "Rust IO(`platform/`, 테스트는 core와 가짜 trait으로)", `tasks/agent_detection::run`

설계는 IO 쪽을 "core와 가짜 trait으로" 테스트한다고 했지만, 시그니처에는 trait도, 아래 규칙을 담은 순수 함수도 없습니다. 그래서 다음은 테스트하지 못했습니다.

- 셸 실행 결과(띄우기 실패, 시간 초과, 0이 아닌 종료 코드, 파싱 실패)를 `ShellEnvFailure`로 바꾸고, 실패면 앱 환경을 쓰는 규칙(FR-AGENT-016, FR-AGENT-018)
- 명령 프롬프트를 고르면 셸을 띄우지 않고 실패도 없다는 규칙(FR-AGENT-014 시나리오 3, FR-AGENT-018 시나리오 3)
- 셸 읽기에 쓴 시간을 `DETECTION_LIMIT`에서 뺀 감지 남은 시간(SC-AGENT-001, SC-AGENT-003)
- 모델 조회와 로그인 확인이 셸을 새로 띄우지 않고 마지막으로 읽은 환경(없으면 앱 환경)을 쓴다는 규칙(FR-AGENT-015)

이 규칙들의 시그니처(예: `fn shell_env_outcome(result: Result<Output, RunError>, ...) -> (ShellEnv, Option<ShellEnvFailure>)`, 셸 실행 trait, 남은 시간 계산 함수)를 설계에 더해 주시거나, 이 부분은 단위 테스트 없이 간다고 답해 주세요.

**답**: 순수 함수를 설계에 추가함. `shell::env`에 `ShellRun`, `shell_env_outcome`, `reads_shell`, `remaining_limit`를, `state.rs`에 `ShellEnvState`의 `set`, `env_or`, `shell`을 추가함(plan.md 시그니처)

### TQ3. 셸 환경 실패 안내와 셸 선택지 설명의 정확한 문구가 있나요?

- 근거: FR-AGENT-018, FR-START-011, plan.md `## 설계` > 시그니처 `detectionView.ts`의 `shellEnvNotice`, `shell.ts`의 `shellDescription`

스펙에는 "셸 설정 파일의 환경 변수가 반영되지 않았다는 안내"와 "그 셸의 설정 파일에 적은 환경 변수가 전달되는지 짧게"만 있어서, 지금은 이유 문구(시간 초과, 셸 실행 실패, 셸 오류 종료, 알아볼 수 없는 출력)가 들어 있는지, 설명이 비어 있지 않은 한 줄인지만 확인합니다. 정확한 문구를 고정하려면 알려 주세요. 또 `shellEnvNotice`가 돌려주는 문자열에 안내 문장까지 들어가는지, 이유 문구만 들어가고 안내 문장은 컴포넌트가 붙이는지도 알려 주세요.

**답**: 아래 문구로 고정함. `shellEnvNotice`는 이유까지 든 문장 전체를 돌려줌. 실패 안내는 `터미널 설정을 읽지 못해 터미널과 결과가 다를 수 있습니다 (<이유>)`이고, 이유는 timeout `시간 초과`, launchFailed `셸을 열지 못함`, exitedWithError `셸 설정 오류`, unreadableOutput `결과를 읽지 못함`임. `shellDescription`은 powerShell `터미널의 PowerShell과 같은 설정으로 실행합니다`, cmd `Windows 기본 설정으로만 실행합니다`, gitBash `터미널의 Git Bash와 같은 설정으로 실행합니다`임

### TQ4. Git Bash 후보를 PATH에서 찾을 때 대소문자와 끝 구분자를 어떻게 다루나요?

- 근거: FR-START-011, plan.md `## 설계` > 시그니처 `choice.rs`의 `git_bash_candidates`

`PATH`의 `...\Git\cmd`에서 `...\Git\bin\bash.exe`를 만든다고 되어 있습니다. 아래 경우에도 후보를 만들어야 하는지 알려 주세요. 지금은 `...\Git\cmd`와 정확히 같은 모양만 테스트했습니다.

- 대소문자가 다른 항목(`C:\Program Files\git\CMD`)
- 끝에 `\`가 붙은 항목(`C:\Program Files\Git\cmd\`)
- `ProgramFiles` 값 끝에 `\`가 붙은 경우(`C:\Program Files\`)
- 경로 비교로 중복을 없앨 때 대소문자를 무시하는지

**답**: 모두 허용함. Windows 경로는 대소문자를 구분하지 않으므로 `git\CMD`, 끝에 `\`가 붙은 `Git\cmd\`, 끝에 `\`가 붙은 `ProgramFiles`(`C:\Program Files\`)도 후보로 봄. 이때 구분자는 하나만 남김. 중복도 대소문자를 무시하고 없애며, 먼저 나온 표기를 남김
