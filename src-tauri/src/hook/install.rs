use super::domain::claude::merge_claude_settings;
use super::domain::codex::{merge_codex_config, merge_codex_hooks};
use super::domain::command::{hook_events, hook_handler};
use super::domain::model::{
    InstallFailure, InstallStatus, InstallTarget, Merge, MergeError, ShellFamily,
};
use super::fs::ConfigFs;
use crate::agent::domain::model::AgentKind;
use serde_json::Value;
use std::io;

/// sh 훅 스크립트.
pub const POSIX_SCRIPT: &str = include_str!("../../resources/hooks/namaste-hook.sh");
/// Windows cmd 훅 스크립트. `namaste-hook.cmd <agent> <Event> [표시]`로 부르면 스크립트 폴더의 상위 폴더에 있는
/// 엔드포인트 파일을 `KEY=VALUE` 줄로 읽고 `curl.exe`로 입력을 보낸다. 출력 없이 항상 0으로 끝난다.
/// cmd는 배치 파일을 시스템 코드 페이지로 읽어 한글이 줄 해석을 깨뜨릴 수 있으므로 파일 안에는 ASCII만 둔다.
pub const WINDOWS_SCRIPT: &str = include_str!("../../resources/hooks/namaste-hook.cmd");

/// 한 대상에 훅을 설치한다. 설정 파일을 먼저 모두 읽어 병합할 수 있는지 확인하고, 그다음 스크립트 → Claude Code
/// `settings.json`, 또는 스크립트 → Codex `config.toml` → `hooks.json` 순서로 바뀐 파일만 쓴다.
/// 바꾼 파일이 없으면 `AlreadyInstalled`, 하나라도 쓰면 `Installed`, 실패하면 `Failed`다.
/// 확인 단계에서 실패하면(설정 파일 손상, Codex 훅 기능 꺼짐) 스크립트를 포함해 아무 파일도 쓰지 않는다.
pub async fn install<F: ConfigFs>(
    fs: &F,
    target: &InstallTarget,
    shell: ShellFamily,
) -> InstallStatus {
    match install_files(fs, target, shell).await {
        Ok(true) => InstallStatus::Installed,
        Ok(false) => InstallStatus::AlreadyInstalled,
        Err(reason) => InstallStatus::Failed { reason },
    }
}

/// 훅 스크립트 권한. 그 사용자만 읽고 실행하고 고칠 수 있다.
const SCRIPT_MODE: u32 = 0o700;
/// 새로 만드는 설정 파일 권한. 이미 있는 파일은 `ConfigFs`가 원래 권한을 지킨다.
const CONFIG_MODE: u32 = 0o644;

/// 쓸 파일 하나.
struct FileWrite {
    path: String,
    contents: String,
    mode: u32,
}

/// 바꾼 파일이 있으면 `Ok(true)`.
async fn install_files<F: ConfigFs>(
    fs: &F,
    target: &InstallTarget,
    shell: ShellFamily,
) -> Result<bool, InstallFailure> {
    let path = |parts: &[&str]| join(shell, &target.home, parts);
    let desired: Vec<(&str, Value)> = hook_events(target.agent, &target.version)
        .into_iter()
        .map(|event| (event, hook_handler(target, shell, event)))
        .collect();

    let (script_path, script) = match shell {
        ShellFamily::Posix => (
            path(&[".namaste-world", "hooks", "namaste-hook.sh"]),
            POSIX_SCRIPT,
        ),
        ShellFamily::Windows => (
            path(&[".namaste-world", "hooks", "namaste-hook.cmd"]),
            WINDOWS_SCRIPT,
        ),
    };
    let mut writes = Vec::new();
    if read(fs, &script_path).await?.as_deref() != Some(script) {
        writes.push(FileWrite {
            path: script_path,
            contents: script.to_owned(),
            mode: SCRIPT_MODE,
        });
    }

    match target.agent {
        AgentKind::Claude => {
            let settings = path(&[".claude", "settings.json"]);
            let current = read(fs, &settings).await?;
            let merge = merge_claude_settings(current.as_deref(), &desired)
                .map_err(|error| merge_failure(error, &settings))?;
            push_update(&mut writes, settings, merge);
        }
        AgentKind::Codex => {
            let hooks_json = path(&[".codex", "hooks.json"]);
            let config_toml = path(&[".codex", "config.toml"]);
            let current_hooks = read(fs, &hooks_json).await?;
            let current_config = read(fs, &config_toml).await?;
            let (hooks_merge, slots) = merge_codex_hooks(current_hooks.as_deref(), &desired)
                .map_err(|error| merge_failure(error, &hooks_json))?;
            let config_merge = merge_codex_config(current_config.as_deref(), &hooks_json, &slots)
                .map_err(|error| merge_failure(error, &config_toml))?;
            // 신뢰 정보를 먼저 써서, hooks.json 쓰기 전에 끊겨도 신뢰 없는 앱 훅이 남지 않게 한다
            push_update(&mut writes, config_toml, config_merge);
            push_update(&mut writes, hooks_json, hooks_merge);
        }
    }

    for write in &writes {
        fs.write(&write.path, &write.contents, write.mode)
            .await
            .map_err(|error| io_failure(&error, &write.path))?;
    }
    Ok(!writes.is_empty())
}

fn push_update(writes: &mut Vec<FileWrite>, path: String, merge: Merge) {
    if let Merge::Updated(contents) = merge {
        writes.push(FileWrite {
            path,
            contents,
            mode: CONFIG_MODE,
        });
    }
}

async fn read<F: ConfigFs>(fs: &F, path: &str) -> Result<Option<String>, InstallFailure> {
    fs.read(path)
        .await
        .map_err(|error| io_failure(&error, path))
}

/// 그 환경의 경로 표기로 홈 아래 경로를 만든다.
fn join(shell: ShellFamily, home: &str, parts: &[&str]) -> String {
    let separator = match shell {
        ShellFamily::Posix => "/",
        ShellFamily::Windows => "\\",
    };
    std::iter::once(home)
        .chain(parts.iter().copied())
        .collect::<Vec<_>>()
        .join(separator)
}

fn merge_failure(error: MergeError, file: &str) -> InstallFailure {
    match error {
        MergeError::HooksDisabled => InstallFailure::CodexHooksDisabled {
            file: file.to_owned(),
        },
        MergeError::InvalidJson | MergeError::InvalidToml | MergeError::NotObject => {
            InstallFailure::InvalidConfig {
                file: file.to_owned(),
            }
        }
    }
}

fn io_failure(error: &io::Error, file: &str) -> InstallFailure {
    match error.kind() {
        io::ErrorKind::PermissionDenied => InstallFailure::NotWritable {
            file: file.to_owned(),
        },
        _ => InstallFailure::Io {
            message: format!("{file}: {error}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::domain::model::{AgentKind, Environment};
    use crate::hook::domain::codex::codex_hook_hash;
    use crate::hook::domain::command::{hook_events, hook_handler, HOOK_MARKER};
    use crate::hook::domain::model::InstallFailure;
    use serde_json::{json, Value};
    use std::collections::{BTreeMap, BTreeSet};
    use std::future::Future;
    use std::io;
    use std::sync::Mutex;
    use toml_edit::DocumentMut;

    /// 경로 → 내용 맵으로 된 메모리 파일 시스템. 쓰기를 거부할 경로와 읽기·쓰기 기록을 가진다.
    #[derive(Default)]
    struct MemFs {
        files: Mutex<BTreeMap<String, String>>,
        denied: BTreeSet<String>,
        reads: Mutex<Vec<String>>,
        writes: Mutex<Vec<(String, u32)>>,
    }

    impl MemFs {
        fn with(files: &[(&str, &str)]) -> Self {
            MemFs {
                files: Mutex::new(
                    files
                        .iter()
                        .map(|(p, c)| (p.to_string(), c.to_string()))
                        .collect(),
                ),
                ..MemFs::default()
            }
        }

        fn deny(mut self, path: &str) -> Self {
            self.denied.insert(path.to_string());
            self
        }

        fn get(&self, path: &str) -> Option<String> {
            self.files.lock().unwrap().get(path).cloned()
        }

        fn snapshot(&self) -> BTreeMap<String, String> {
            self.files.lock().unwrap().clone()
        }

        fn written(&self) -> Vec<(String, u32)> {
            self.writes.lock().unwrap().clone()
        }

        fn written_paths(&self) -> Vec<String> {
            self.written().into_iter().map(|(p, _)| p).collect()
        }

        fn read_paths(&self) -> Vec<String> {
            self.reads.lock().unwrap().clone()
        }

        fn clear_log(&self) {
            self.reads.lock().unwrap().clear();
            self.writes.lock().unwrap().clear();
        }
    }

    impl ConfigFs for MemFs {
        fn read(&self, path: &str) -> impl Future<Output = io::Result<Option<String>>> + Send {
            self.reads.lock().unwrap().push(path.to_string());
            let found = self.get(path);
            async move { Ok(found) }
        }

        fn write(
            &self,
            path: &str,
            contents: &str,
            mode: u32,
        ) -> impl Future<Output = io::Result<()>> + Send {
            let result = if self.denied.contains(path) {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            } else {
                self.writes.lock().unwrap().push((path.to_string(), mode));
                self.files
                    .lock()
                    .unwrap()
                    .insert(path.to_string(), contents.to_string());
                Ok(())
            };
            async move { result }
        }
    }

    const MAC_HOME: &str = "/Users/tester";
    const WSL_HOME: &str = "/home/tester";
    const WIN_HOME: &str = "C:\\Users\\홍 길동";

    /// 한 대상이 쓰는 파일 경로.
    struct Paths {
        script: String,
        claude_settings: String,
        codex_hooks: String,
        codex_config: String,
    }

    fn posix_paths(home: &str) -> Paths {
        Paths {
            script: format!("{home}/.namaste-world/hooks/namaste-hook.sh"),
            claude_settings: format!("{home}/.claude/settings.json"),
            codex_hooks: format!("{home}/.codex/hooks.json"),
            codex_config: format!("{home}/.codex/config.toml"),
        }
    }

    fn windows_paths(home: &str) -> Paths {
        Paths {
            script: format!("{home}\\.namaste-world\\hooks\\namaste-hook.cmd"),
            claude_settings: format!("{home}\\.claude\\settings.json"),
            codex_hooks: format!("{home}\\.codex\\hooks.json"),
            codex_config: format!("{home}\\.codex\\config.toml"),
        }
    }

    fn mac(agent: AgentKind, version: &str) -> InstallTarget {
        InstallTarget {
            agent,
            environment: Environment::Macos,
            version: version.to_string(),
            home: MAC_HOME.to_string(),
            endpoint_path: format!("{MAC_HOME}/.namaste-world/endpoint"),
        }
    }

    fn wsl(agent: AgentKind, version: &str) -> InstallTarget {
        InstallTarget {
            agent,
            environment: Environment::Wsl {
                distro: "Ubuntu".to_string(),
            },
            version: version.to_string(),
            home: WSL_HOME.to_string(),
            endpoint_path: "/mnt/c/Users/홍 길동/.namaste-world/endpoint".to_string(),
        }
    }

    fn win(agent: AgentKind, version: &str) -> InstallTarget {
        InstallTarget {
            agent,
            environment: Environment::Windows,
            version: version.to_string(),
            home: WIN_HOME.to_string(),
            endpoint_path: format!("{WIN_HOME}\\.namaste-world\\endpoint"),
        }
    }

    const CLAUDE_VERSION: &str = "2.1.325";
    const CODEX_VERSION: &str = "0.146.0";

    fn failed(reason: InstallFailure) -> InstallStatus {
        InstallStatus::Failed { reason }
    }

    fn is_app_handler(handler: &Value) -> bool {
        handler.to_string().contains(HOOK_MARKER)
    }

    /// 이벤트 배열에서 앱 handler를 가진 group의 위치와 handler를 모두 찾는다.
    fn app_groups(hooks: &Value, event: &str) -> Vec<(usize, Value)> {
        hooks
            .get(event)
            .and_then(Value::as_array)
            .map(|groups| {
                groups
                    .iter()
                    .enumerate()
                    .flat_map(|(i, group)| {
                        group
                            .get("hooks")
                            .and_then(Value::as_array)
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .filter(is_app_handler)
                            .map(move |h| (i, h))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 설정 JSON의 `hooks`에 그 대상의 이벤트마다 앱 handler가 정확히 하나씩 들어 있는지 확인하고,
    /// 이벤트별 앱 group 위치를 돌려준다.
    fn assert_hooks_registered(
        text: &str,
        target: &InstallTarget,
        shell: ShellFamily,
    ) -> Vec<(&'static str, usize, Value)> {
        let root: Value = serde_json::from_str(text).expect("settings is JSON");
        let hooks = root.get("hooks").expect("hooks key");
        let events = hook_events(target.agent, &target.version);
        assert!(!events.is_empty());
        let placed: Vec<_> = events
            .iter()
            .map(|event| {
                let found = app_groups(hooks, event);
                assert_eq!(found.len(), 1, "{event}: app handler count");
                let (index, handler) = found.into_iter().next().unwrap();
                assert_eq!(handler, hook_handler(target, shell, event), "{event}");
                (*event, index, handler)
            })
            .collect();
        let registered: BTreeSet<&str> = hooks
            .as_object()
            .expect("hooks object")
            .keys()
            .filter(|k| !app_groups(hooks, k).is_empty())
            .map(String::as_str)
            .collect();
        assert_eq!(registered, events.iter().copied().collect::<BTreeSet<_>>());
        placed
    }

    fn snake(event: &str) -> String {
        event
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                let lower = c.to_ascii_lowercase();
                if c.is_ascii_uppercase() && i > 0 {
                    vec!['_', lower]
                } else {
                    vec![lower]
                }
            })
            .collect()
    }

    /// config.toml에 훅 기능이 켜져 있고, 앱 group마다 신뢰 해시가 들어 있는지 확인한다.
    fn assert_codex_trusted(
        config: &str,
        hooks_json_path: &str,
        placed: &[(&'static str, usize, Value)],
    ) {
        let doc: DocumentMut = config.parse().expect("config is TOML");
        let hooks_on = doc
            .get("features")
            .and_then(|f| f.get("hooks"))
            .and_then(|v| v.as_bool());
        assert_eq!(hooks_on, Some(true));
        for (event, index, handler) in placed {
            let key = format!("{hooks_json_path}:{}:{index}:0", snake(event));
            let hash = doc
                .get("hooks")
                .and_then(|h| h.get("state"))
                .and_then(|s| s.get(&key))
                .and_then(|e| e.get("trusted_hash"))
                .and_then(|v| v.as_str());
            assert_eq!(
                hash,
                Some(codex_hook_hash(event, handler).as_str()),
                "trust key {key}"
            );
        }
    }

    #[tokio::test]
    async fn fr_hook_002_first_run_installs_claude_script_then_settings_on_macos() {
        let fs = MemFs::default();
        let target = mac(AgentKind::Claude, CLAUDE_VERSION);
        let p = posix_paths(MAC_HOME);

        let status = install(&fs, &target, ShellFamily::Posix).await;

        assert_eq!(status, InstallStatus::Installed);
        assert_eq!(
            fs.written_paths(),
            vec![p.script.clone(), p.claude_settings.clone()]
        );
        assert_eq!(fs.get(&p.script).as_deref(), Some(POSIX_SCRIPT));
        let settings = fs.get(&p.claude_settings).expect("settings created");
        assert_hooks_registered(&settings, &target, ShellFamily::Posix);
    }

    #[tokio::test]
    async fn fr_hook_006_posix_script_is_written_with_mode_0700() {
        let fs = MemFs::default();
        let p = posix_paths(MAC_HOME);

        install(
            &fs,
            &mac(AgentKind::Claude, CLAUDE_VERSION),
            ShellFamily::Posix,
        )
        .await;

        let script_modes: Vec<u32> = fs
            .written()
            .into_iter()
            .filter(|(path, _)| *path == p.script)
            .map(|(_, mode)| mode)
            .collect();
        assert_eq!(script_modes, vec![0o700]);
    }

    #[tokio::test]
    async fn fr_hook_002_first_run_installs_codex_script_then_config_then_hooks_json_on_macos() {
        let fs = MemFs::default();
        let target = mac(AgentKind::Codex, CODEX_VERSION);
        let p = posix_paths(MAC_HOME);

        let status = install(&fs, &target, ShellFamily::Posix).await;

        assert_eq!(status, InstallStatus::Installed);
        assert_eq!(
            fs.written_paths(),
            vec![
                p.script.clone(),
                p.codex_config.clone(),
                p.codex_hooks.clone()
            ]
        );
        assert_eq!(fs.get(&p.script).as_deref(), Some(POSIX_SCRIPT));
        let hooks = fs.get(&p.codex_hooks).expect("hooks.json created");
        let placed = assert_hooks_registered(&hooks, &target, ShellFamily::Posix);
        let config = fs.get(&p.codex_config).expect("config.toml created");
        assert_codex_trusted(&config, &p.codex_hooks, &placed);
    }

    #[tokio::test]
    async fn fr_hook_003_codex_0_169_registers_and_trusts_every_event() {
        let fs = MemFs::default();
        let target = mac(AgentKind::Codex, "0.169.0");
        let p = posix_paths(MAC_HOME);

        let status = install(&fs, &target, ShellFamily::Posix).await;

        assert_eq!(status, InstallStatus::Installed);
        let hooks = fs.get(&p.codex_hooks).expect("hooks.json created");
        let placed = assert_hooks_registered(&hooks, &target, ShellFamily::Posix);
        assert_eq!(placed.len(), 14);
        assert_codex_trusted(&fs.get(&p.codex_config).unwrap(), &p.codex_hooks, &placed);
    }

    #[tokio::test]
    async fn fr_hook_002_second_run_writes_nothing_and_reports_already_installed() {
        let cases = [
            (mac(AgentKind::Claude, CLAUDE_VERSION), ShellFamily::Posix),
            (mac(AgentKind::Codex, CODEX_VERSION), ShellFamily::Posix),
            (wsl(AgentKind::Codex, CODEX_VERSION), ShellFamily::Posix),
            (win(AgentKind::Claude, CLAUDE_VERSION), ShellFamily::Windows),
            (win(AgentKind::Codex, CODEX_VERSION), ShellFamily::Windows),
        ];
        for (target, shell) in cases {
            let fs = MemFs::default();
            assert_eq!(install(&fs, &target, shell).await, InstallStatus::Installed);
            let after_first = fs.snapshot();
            fs.clear_log();

            let status = install(&fs, &target, shell).await;

            assert_eq!(status, InstallStatus::AlreadyInstalled, "{target:?}");
            assert_eq!(fs.written(), vec![], "{target:?}");
            assert_eq!(fs.snapshot(), after_first, "{target:?}");
        }
    }

    #[tokio::test]
    async fn fr_hook_002_only_files_that_differ_are_written() {
        let p = posix_paths(MAC_HOME);
        let fs = MemFs::with(&[(&p.script, POSIX_SCRIPT)]);
        let target = mac(AgentKind::Claude, CLAUDE_VERSION);

        let status = install(&fs, &target, ShellFamily::Posix).await;

        assert_eq!(status, InstallStatus::Installed);
        assert_eq!(fs.written_paths(), vec![p.claude_settings.clone()]);
    }

    #[tokio::test]
    async fn fr_hook_002_stale_app_hook_and_script_are_replaced_without_duplicates() {
        let p = posix_paths(MAC_HOME);
        let user_group = json!({"hooks": [{"type": "command", "command": "say done"}]});
        let old_app_group = json!({"hooks": [{
            "type": "command",
            "command": format!(": {HOOK_MARKER}; /bin/sh /old/namaste-hook.sh claude Stop"),
            "timeout": 30
        }]});
        let existing = json!({"hooks": {"Stop": [user_group.clone(), old_app_group]}}).to_string();
        let fs = MemFs::with(&[
            (&p.script, "#!/bin/sh\n# old version\n"),
            (&p.claude_settings, &existing),
        ]);
        let target = mac(AgentKind::Claude, CLAUDE_VERSION);

        let status = install(&fs, &target, ShellFamily::Posix).await;

        assert_eq!(status, InstallStatus::Installed);
        assert_eq!(fs.get(&p.script).as_deref(), Some(POSIX_SCRIPT));
        let settings = fs.get(&p.claude_settings).unwrap();
        assert_hooks_registered(&settings, &target, ShellFamily::Posix);
        let root: Value = serde_json::from_str(&settings).unwrap();
        assert_eq!(root["hooks"]["Stop"][0], user_group);
    }

    #[tokio::test]
    async fn fr_hook_004_other_hooks_and_settings_are_kept() {
        let p = posix_paths(MAC_HOME);
        let user_group = json!({"hooks": [{"type": "command", "command": "say done"}]});
        let orca_group =
            json!({"matcher": "", "hooks": [{"type": "command", "command": "orca-hook stop"}]});
        let existing = json!({
            "model": "opus",
            "permissions": {"allow": ["Bash(ls:*)"]},
            "hooks": {"Stop": [user_group.clone(), orca_group.clone()]}
        })
        .to_string();
        let fs = MemFs::with(&[(&p.claude_settings, &existing)]);
        let target = mac(AgentKind::Claude, CLAUDE_VERSION);

        let status = install(&fs, &target, ShellFamily::Posix).await;

        assert_eq!(status, InstallStatus::Installed);
        let root: Value = serde_json::from_str(&fs.get(&p.claude_settings).unwrap()).unwrap();
        assert_eq!(root["model"], json!("opus"));
        assert_eq!(root["permissions"], json!({"allow": ["Bash(ls:*)"]}));
        assert_eq!(root["hooks"]["Stop"][0], user_group);
        assert_eq!(root["hooks"]["Stop"][1], orca_group);
        assert_eq!(app_groups(&root["hooks"], "Stop").len(), 1);
    }

    #[tokio::test]
    async fn fr_hook_004_invalid_claude_settings_is_left_as_is_and_fails() {
        let p = posix_paths(MAC_HOME);
        let broken = "{ \"model\": \"opus\", ";
        let fs = MemFs::with(&[(&p.claude_settings, broken)]);

        let status = install(
            &fs,
            &mac(AgentKind::Claude, CLAUDE_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::InvalidConfig {
                file: p.claude_settings.clone()
            })
        );
        assert_eq!(fs.get(&p.claude_settings).as_deref(), Some(broken));
        assert!(!fs.written_paths().contains(&p.claude_settings));
    }

    #[tokio::test]
    async fn fr_hook_004_claude_settings_that_is_not_an_object_fails() {
        let p = posix_paths(MAC_HOME);
        let fs = MemFs::with(&[(&p.claude_settings, "[1, 2]")]);

        let status = install(
            &fs,
            &mac(AgentKind::Claude, CLAUDE_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::InvalidConfig {
                file: p.claude_settings.clone()
            })
        );
        assert_eq!(fs.get(&p.claude_settings).as_deref(), Some("[1, 2]"));
    }

    #[tokio::test]
    async fn fr_hook_006_unwritable_claude_settings_fails_with_its_path() {
        let p = posix_paths(MAC_HOME);
        let existing = "{\"model\": \"opus\"}";
        let fs = MemFs::with(&[(&p.claude_settings, existing)]).deny(&p.claude_settings);

        let status = install(
            &fs,
            &mac(AgentKind::Claude, CLAUDE_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::NotWritable {
                file: p.claude_settings.clone()
            })
        );
        assert_eq!(fs.get(&p.claude_settings).as_deref(), Some(existing));
    }

    #[tokio::test]
    async fn fr_hook_006_unwritable_script_fails_with_its_path_and_settings_are_not_written() {
        let p = posix_paths(MAC_HOME);
        let fs = MemFs::default().deny(&p.script);

        let status = install(
            &fs,
            &mac(AgentKind::Claude, CLAUDE_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::NotWritable {
                file: p.script.clone()
            })
        );
        assert_eq!(fs.written(), vec![]);
    }

    #[tokio::test]
    async fn fr_hook_006_unwritable_codex_hooks_json_fails_with_its_path() {
        let p = posix_paths(MAC_HOME);
        let existing = "{\"hooks\": {}}";
        let fs = MemFs::with(&[(&p.codex_hooks, existing)]).deny(&p.codex_hooks);

        let status = install(
            &fs,
            &mac(AgentKind::Codex, CODEX_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::NotWritable {
                file: p.codex_hooks.clone()
            })
        );
        assert_eq!(fs.get(&p.codex_hooks).as_deref(), Some(existing));
    }

    #[tokio::test]
    async fn fr_hook_006_unwritable_codex_config_fails_with_its_path() {
        let p = posix_paths(MAC_HOME);
        let existing = "model = \"gpt-5\"\n";
        let fs = MemFs::with(&[(&p.codex_config, existing)]).deny(&p.codex_config);

        let status = install(
            &fs,
            &mac(AgentKind::Codex, CODEX_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::NotWritable {
                file: p.codex_config.clone()
            })
        );
        assert_eq!(fs.get(&p.codex_config).as_deref(), Some(existing));
    }

    #[tokio::test]
    async fn fr_hook_006_unwritable_codex_config_leaves_hooks_json_unwritten() {
        let p = posix_paths(MAC_HOME);
        let existing_hooks = "{\"hooks\": {}}";
        let existing_config = "model = \"gpt-5\"\n";
        let fs = MemFs::with(&[
            (&p.codex_hooks, existing_hooks),
            (&p.codex_config, existing_config),
        ])
        .deny(&p.codex_config);

        let status = install(
            &fs,
            &mac(AgentKind::Codex, CODEX_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::NotWritable {
                file: p.codex_config.clone()
            })
        );
        assert!(!fs.written_paths().contains(&p.codex_hooks));
        assert_eq!(fs.get(&p.codex_hooks).as_deref(), Some(existing_hooks));
    }

    #[tokio::test]
    async fn fr_hook_006_new_codex_hooks_json_is_not_created_when_config_is_unwritable() {
        let p = posix_paths(MAC_HOME);
        let existing = "model = \"gpt-5\"\n";
        let fs = MemFs::with(&[(&p.codex_config, existing)]).deny(&p.codex_config);

        install(
            &fs,
            &mac(AgentKind::Codex, CODEX_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(fs.get(&p.codex_hooks), None);
    }

    #[tokio::test]
    async fn fr_hook_007_claude_installs_even_when_codex_cannot_be_written() {
        let p = posix_paths(MAC_HOME);
        let fs = MemFs::with(&[(&p.codex_hooks, "{}")]).deny(&p.codex_hooks);
        let claude = mac(AgentKind::Claude, CLAUDE_VERSION);
        let codex = mac(AgentKind::Codex, CODEX_VERSION);

        let claude_status = install(&fs, &claude, ShellFamily::Posix).await;
        let codex_status = install(&fs, &codex, ShellFamily::Posix).await;

        assert_eq!(claude_status, InstallStatus::Installed);
        assert_eq!(
            codex_status,
            failed(InstallFailure::NotWritable {
                file: p.codex_hooks.clone()
            })
        );
        assert_eq!(fs.get(&p.script).as_deref(), Some(POSIX_SCRIPT));
        assert_hooks_registered(
            &fs.get(&p.claude_settings).unwrap(),
            &claude,
            ShellFamily::Posix,
        );
    }

    #[tokio::test]
    async fn fr_hook_006_codex_hooks_turned_off_by_user_leaves_codex_files_untouched() {
        let p = posix_paths(MAC_HOME);
        for config in [
            "# 사용자 설정\n[features]\nhooks = false\n",
            "[features]\ncodex_hooks = false\n",
        ] {
            let fs = MemFs::with(&[(&p.codex_config, config)]);

            let status = install(
                &fs,
                &mac(AgentKind::Codex, CODEX_VERSION),
                ShellFamily::Posix,
            )
            .await;

            assert_eq!(
                status,
                failed(InstallFailure::CodexHooksDisabled {
                    file: p.codex_config.clone()
                }),
                "{config}"
            );
            let written = fs.written_paths();
            assert!(!written.contains(&p.codex_hooks), "{config}");
            assert!(!written.contains(&p.codex_config), "{config}");
            assert_eq!(fs.get(&p.codex_config).as_deref(), Some(config));
            assert_eq!(fs.get(&p.codex_hooks), None);
        }
    }

    #[tokio::test]
    async fn fr_hook_004_invalid_codex_hooks_json_is_left_as_is_and_config_is_not_written() {
        let p = posix_paths(MAC_HOME);
        let broken = "{\"hooks\": ";
        let fs = MemFs::with(&[(&p.codex_hooks, broken)]);

        let status = install(
            &fs,
            &mac(AgentKind::Codex, CODEX_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::InvalidConfig {
                file: p.codex_hooks.clone()
            })
        );
        assert_eq!(fs.get(&p.codex_hooks).as_deref(), Some(broken));
        let written = fs.written_paths();
        assert!(!written.contains(&p.codex_hooks));
        assert!(!written.contains(&p.codex_config));
    }

    #[tokio::test]
    async fn fr_hook_004_invalid_codex_config_is_left_as_is_and_fails() {
        let p = posix_paths(MAC_HOME);
        let broken = "[features\nhooks = ";
        let fs = MemFs::with(&[(&p.codex_config, broken)]);

        let status = install(
            &fs,
            &mac(AgentKind::Codex, CODEX_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::InvalidConfig {
                file: p.codex_config.clone()
            })
        );
        assert_eq!(fs.get(&p.codex_config).as_deref(), Some(broken));
        assert!(!fs.written_paths().contains(&p.codex_config));
    }

    #[tokio::test]
    async fn fr_hook_002_only_user_global_files_of_that_agent_are_read_or_written() {
        let p = posix_paths(MAC_HOME);
        let project_claude = format!("{MAC_HOME}/work/app/.claude/settings.json");
        let project_codex = format!("{MAC_HOME}/work/app/.codex/hooks.json");
        let cases = [
            (
                AgentKind::Claude,
                CLAUDE_VERSION,
                BTreeSet::from([p.script.clone(), p.claude_settings.clone()]),
            ),
            (
                AgentKind::Codex,
                CODEX_VERSION,
                BTreeSet::from([
                    p.script.clone(),
                    p.codex_hooks.clone(),
                    p.codex_config.clone(),
                ]),
            ),
        ];
        for (agent, version, allowed) in cases {
            let fs = MemFs::with(&[(&project_claude, "{}"), (&project_codex, "{}")]);

            let status = install(&fs, &mac(agent, version), ShellFamily::Posix).await;

            assert_eq!(status, InstallStatus::Installed, "{agent:?}");
            let touched: BTreeSet<String> = fs
                .read_paths()
                .into_iter()
                .chain(fs.written_paths())
                .collect();
            assert!(touched.is_subset(&allowed), "{agent:?}: {touched:?}");
            assert_eq!(fs.get(&project_claude).as_deref(), Some("{}"));
            assert_eq!(fs.get(&project_codex).as_deref(), Some("{}"));
        }
    }

    #[tokio::test]
    async fn fr_hook_002_wsl_install_goes_to_the_distro_home() {
        let fs = MemFs::default();
        let target = wsl(AgentKind::Claude, CLAUDE_VERSION);
        let p = posix_paths(WSL_HOME);

        let status = install(&fs, &target, ShellFamily::Posix).await;

        assert_eq!(status, InstallStatus::Installed);
        assert_eq!(
            fs.written_paths(),
            vec![p.script.clone(), p.claude_settings.clone()]
        );
        assert_eq!(fs.written()[0].1, 0o700);
        assert_eq!(fs.get(&p.script).as_deref(), Some(POSIX_SCRIPT));
        assert_hooks_registered(
            &fs.get(&p.claude_settings).unwrap(),
            &target,
            ShellFamily::Posix,
        );
    }

    #[tokio::test]
    async fn fr_hook_011_windows_claude_installs_cmd_script_and_settings_under_user_profile() {
        let fs = MemFs::default();
        let target = win(AgentKind::Claude, CLAUDE_VERSION);
        let p = windows_paths(WIN_HOME);

        let status = install(&fs, &target, ShellFamily::Windows).await;

        assert_eq!(status, InstallStatus::Installed);
        assert_eq!(
            fs.written_paths(),
            vec![p.script.clone(), p.claude_settings.clone()]
        );
        assert_eq!(fs.get(&p.script).as_deref(), Some(WINDOWS_SCRIPT));
        assert_hooks_registered(
            &fs.get(&p.claude_settings).unwrap(),
            &target,
            ShellFamily::Windows,
        );
    }

    #[tokio::test]
    async fn fr_hook_011_windows_codex_trusts_hooks_under_windows_hooks_json_path() {
        let fs = MemFs::default();
        let target = win(AgentKind::Codex, CODEX_VERSION);
        let p = windows_paths(WIN_HOME);

        let status = install(&fs, &target, ShellFamily::Windows).await;

        assert_eq!(status, InstallStatus::Installed);
        assert_eq!(
            fs.written_paths(),
            vec![
                p.script.clone(),
                p.codex_config.clone(),
                p.codex_hooks.clone()
            ]
        );
        assert_eq!(fs.get(&p.script).as_deref(), Some(WINDOWS_SCRIPT));
        let placed = assert_hooks_registered(
            &fs.get(&p.codex_hooks).unwrap(),
            &target,
            ShellFamily::Windows,
        );
        assert_codex_trusted(&fs.get(&p.codex_config).unwrap(), &p.codex_hooks, &placed);
    }

    #[tokio::test]
    async fn fr_hook_011_codex_on_windows_and_wsl_are_installed_separately() {
        let fs = MemFs::default();
        let windows = win(AgentKind::Codex, CODEX_VERSION);
        let ubuntu = wsl(AgentKind::Codex, CODEX_VERSION);
        let wp = windows_paths(WIN_HOME);
        let up = posix_paths(WSL_HOME);

        let w = install(&fs, &windows, ShellFamily::Windows).await;
        let u = install(&fs, &ubuntu, ShellFamily::Posix).await;

        assert_eq!((w, u), (InstallStatus::Installed, InstallStatus::Installed));
        assert_hooks_registered(
            &fs.get(&wp.codex_hooks).unwrap(),
            &windows,
            ShellFamily::Windows,
        );
        assert_hooks_registered(
            &fs.get(&up.codex_hooks).unwrap(),
            &ubuntu,
            ShellFamily::Posix,
        );
    }

    #[tokio::test]
    async fn fr_hook_004_invalid_claude_settings_writes_no_file_including_script() {
        let p = posix_paths(MAC_HOME);
        let broken = "{ \"model\": \"opus\", ";
        let fs = MemFs::with(&[(&p.claude_settings, broken)]);

        let status = install(
            &fs,
            &mac(AgentKind::Claude, CLAUDE_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::InvalidConfig {
                file: p.claude_settings.clone()
            })
        );
        assert_eq!(fs.written(), vec![]);
        assert_eq!(fs.get(&p.script), None);
    }

    #[tokio::test]
    async fn fr_hook_006_codex_hooks_turned_off_writes_no_file_including_script() {
        let p = posix_paths(MAC_HOME);
        for config in [
            "[features]\nhooks = false\n",
            "[features]\ncodex_hooks = false\n",
        ] {
            let fs = MemFs::with(&[(&p.codex_config, config)]);

            let status = install(
                &fs,
                &mac(AgentKind::Codex, CODEX_VERSION),
                ShellFamily::Posix,
            )
            .await;

            assert_eq!(
                status,
                failed(InstallFailure::CodexHooksDisabled {
                    file: p.codex_config.clone()
                }),
                "{config}"
            );
            assert_eq!(fs.written(), vec![], "{config}");
            assert_eq!(fs.get(&p.script), None, "{config}");
        }
    }

    #[tokio::test]
    async fn fr_hook_004_invalid_codex_hooks_json_writes_no_file_including_script() {
        let p = posix_paths(MAC_HOME);
        let broken = "{\"hooks\": ";
        let fs = MemFs::with(&[(&p.codex_hooks, broken)]);

        let status = install(
            &fs,
            &mac(AgentKind::Codex, CODEX_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::InvalidConfig {
                file: p.codex_hooks.clone()
            })
        );
        assert_eq!(fs.written(), vec![]);
        assert_eq!(fs.get(&p.script), None);
        assert_eq!(fs.get(&p.codex_config), None);
    }

    #[tokio::test]
    async fn fr_hook_004_invalid_codex_config_writes_neither_hooks_json_nor_script() {
        let p = posix_paths(MAC_HOME);
        let broken = "[features\nhooks = ";
        let fs = MemFs::with(&[(&p.codex_config, broken)]);

        let status = install(
            &fs,
            &mac(AgentKind::Codex, CODEX_VERSION),
            ShellFamily::Posix,
        )
        .await;

        assert_eq!(
            status,
            failed(InstallFailure::InvalidConfig {
                file: p.codex_config.clone()
            })
        );
        assert_eq!(fs.written(), vec![]);
        assert_eq!(fs.get(&p.codex_hooks), None);
        assert_eq!(fs.get(&p.script), None);
        assert_eq!(fs.get(&p.codex_config).as_deref(), Some(broken));
    }

    #[tokio::test]
    async fn fr_hook_006_new_claude_settings_is_written_with_mode_0644_and_script_0700() {
        let cases = [
            (
                mac(AgentKind::Claude, CLAUDE_VERSION),
                ShellFamily::Posix,
                posix_paths(MAC_HOME),
            ),
            (
                win(AgentKind::Claude, CLAUDE_VERSION),
                ShellFamily::Windows,
                windows_paths(WIN_HOME),
            ),
        ];
        for (target, shell, p) in cases {
            let fs = MemFs::default();

            install(&fs, &target, shell).await;

            assert_eq!(
                fs.written(),
                vec![
                    (p.script.clone(), 0o700),
                    (p.claude_settings.clone(), 0o644)
                ],
                "{target:?}"
            );
        }
    }

    #[tokio::test]
    async fn fr_hook_006_new_codex_files_are_written_with_mode_0644_and_script_0700() {
        let cases = [
            (
                mac(AgentKind::Codex, CODEX_VERSION),
                ShellFamily::Posix,
                posix_paths(MAC_HOME),
            ),
            (
                wsl(AgentKind::Codex, CODEX_VERSION),
                ShellFamily::Posix,
                posix_paths(WSL_HOME),
            ),
            (
                win(AgentKind::Codex, CODEX_VERSION),
                ShellFamily::Windows,
                windows_paths(WIN_HOME),
            ),
        ];
        for (target, shell, p) in cases {
            let fs = MemFs::default();

            install(&fs, &target, shell).await;

            assert_eq!(
                fs.written(),
                vec![
                    (p.script.clone(), 0o700),
                    (p.codex_config.clone(), 0o644),
                    (p.codex_hooks.clone(), 0o644),
                ],
                "{target:?}"
            );
        }
    }
}
