# Test Questions

### TQ1. `desired`에 없는 이벤트에 남은 앱 group은 지워야 하나요?

- 근거: FR-HOOK-002 "낡은 훅", plan.md 설계 `merge_claude_settings`와 `merge_codex_hooks`. 예를 들어 Codex를 0.169.0에서 0.168로 내리면 `PreModelRequest`의 앱 group이 `desired`에 없습니다. 설계는 "각 이벤트 배열에서 앱 항목을 뺀 뒤"라고만 해서, `desired`에 없는 이벤트 배열의 앱 group을 지우는지, 이벤트 배열이 비면 키를 지우는지 알 수 없습니다.

**답**: 지웁니다. `desired`에 없는 이벤트 배열에서도 앱 group을 빼고, 그 결과 배열이 비면 이벤트 키도 지웁니다. 앱이 아닌 group이 남아 있으면 키를 그대로 둡니다.

### TQ2. `[features] codex_hooks = true`만 있고 `hooks`가 없으면 `hooks = true`를 더 적나요?

- 근거: FR-HOOK-006 "Codex 훅 기능이 꺼져 있음", plan.md 설계 `merge_codex_config` "[features] hooks(또는 legacy codex_hooks)가 false면 Err(HooksDisabled). 없으면 hooks = true를 넣는다". legacy 키가 true일 때 "없으면"에 해당하는지 알 수 없습니다.

**답**: 더 적지 않습니다. `codex_hooks = true`만 있으면 훅 기능이 켜진 것으로 보고 `[features]`를 바꾸지 않습니다. `hooks`와 `codex_hooks`가 모두 없을 때만 `hooks = true`를 넣습니다.

### TQ3. 뒤 단계에서 실패하면 훅 스크립트는 이미 쓴 채로 두나요?

- 근거: FR-HOOK-004 "손상된 설정 파일", FR-HOOK-006 "사용자가 Codex 훅 기능을 꺼 둠", `install.rs`의 `install` 문서 주석(스크립트 → 설정 파일 순서). 예를 들어 `settings.json`이 올바른 JSON이 아니거나 Codex `[features] hooks = false`일 때, 스크립트(`~/.namaste-world/hooks/namaste-hook.sh`)를 먼저 쓰고 실패로 끝내는지, 설정 파일을 모두 확인한 뒤에야 스크립트를 쓰는지 알 수 없습니다. "Codex 훅 파일을 바꾸지 않고"에 스크립트가 들어가는지도 알 수 없습니다. 지금 테스트는 이 경우 스크립트를 썼는지 확인하지 않습니다.

**답**: 설정을 모두 확인한 뒤 씁니다. 설정 파일을 먼저 읽어 병합할 수 있는지(JSON/TOML 해석, Codex 훅 기능) 확인하고, 실패하면 스크립트를 포함해 어떤 파일도 쓰지 않습니다.

### TQ4. 새로 만드는 설정 파일(`settings.json`, `hooks.json`, `config.toml`)은 어떤 mode로 쓰나요?

- 근거: plan.md 설계 `ConfigFs::write`의 `mode`, FR-HOOK-006 "설정 파일이 없음". 스크립트는 0o700, 엔드포인트 파일은 0600으로 정해져 있지만 설정 파일의 mode는 설계에 없습니다. 기존 파일을 바꿀 때 원래 mode를 유지해야 하는지도 알 수 없습니다(`ConfigFs`에는 mode를 읽는 메서드가 없음). 지금 테스트는 설정 파일의 mode를 확인하지 않습니다.

**답**: Orca와 같게 합니다. 새로 만드는 설정 파일(`settings.json`, `hooks.json`, `config.toml`)은 `0o644`로 쓰고, 이미 있는 파일은 원래 권한을 그대로 둡니다(`ConfigFs::write`의 기본 구현이 기존 파일의 권한을 지킵니다). 훅 스크립트는 설계대로 `0o700`입니다.

### TQ5. Codex `config.toml`을 해석할 수 없으면 `hooks.json`도 쓰지 않나요?

- 근거: FR-HOOK-004 "손상된 설정 파일", `install.rs`의 `install` 문서 주석("Codex `config.toml` 확인 → `hooks.json` → `config.toml`"), plan.md 설계("HooksDisabled면 hooks.json도 쓰지 않는다"). 훅 기능이 꺼진 경우만 `hooks.json`을 쓰지 않는다고 적혀 있고, `config.toml`이 TOML이 아닐 때 `hooks.json`을 먼저 써도 되는지는 적혀 있지 않습니다. 지금 테스트는 이 경우 `config.toml`을 쓰지 않고 `InvalidConfig { file: config.toml 경로 }`인지만 확인합니다.

**답**: 쓰지 않습니다. `config.toml`을 해석할 수 없으면 `hooks.json`과 스크립트도 쓰지 않고 `InvalidConfig { file: config.toml 경로 }`로 남깁니다.
