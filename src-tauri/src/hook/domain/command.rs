use super::model::{InstallTarget, ShellFamily};
use crate::agent::domain::model::{AgentEntry, AgentKind, AgentStatus};
use serde_json::{json, Value};

/// 앱 훅 항목에 들어가는 표시. 이 문자열이 handler의 `command`나 `args`에 있으면 앱 항목이다.
pub const HOOK_MARKER: &str = "namaste-world-hook";

/// Claude Code가 제공하는 훅 이벤트. `WorktreeCreate`, `WorktreeRemove`는 등록만 해도 git worktree 기본 동작을
/// 훅이 대신하고, 훅이 경로를 출력하지 않으면 worktree 생성이 실패하므로 넣지 않는다.
const CLAUDE_EVENTS: [&str; 31] = [
    "SessionStart",
    "Setup",
    "UserPromptSubmit",
    "UserPromptExpansion",
    "PreToolUse",
    "PermissionRequest",
    "PermissionDenied",
    "PostToolUse",
    "PostToolUseFailure",
    "PostToolBatch",
    "Notification",
    "MessageDisplay",
    "SubagentStart",
    "SubagentStop",
    "TaskCreated",
    "TaskCompleted",
    "Stop",
    "StopFailure",
    "TeammateIdle",
    "InstructionsLoaded",
    "ConfigChange",
    "CwdChanged",
    "DirectoryAdded",
    "FileChanged",
    "PreCompact",
    "PostCompact",
    "PreModelSwitch",
    "PostModelSwitch",
    "Elicitation",
    "ElicitationResult",
    "SessionEnd",
];

/// Codex 0.146.0에서 확인한 훅 이벤트.
const CODEX_BASE_EVENTS: [&str; 11] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "PreCompact",
    "PostCompact",
    "SubagentStart",
    "SubagentStop",
    "Stop",
    "SessionEnd",
];

/// Codex 0.169.0부터 더해진 훅 이벤트.
const CODEX_0_169_EVENTS: [&str; 3] = ["PreModelRequest", "PostModelRequest", "Interrupt"];

/// `SessionEnd`는 두 에이전트 모두 짧게만 기다리므로 시간 제한을 따로 둔다(Codex는 최대 3초).
const SESSION_END_TIMEOUT: u64 = 3;
const DEFAULT_TIMEOUT: u64 = 5;

/// 설치할지. 감지 상태가 설치됨인 항목만 설치한다.
pub fn should_install(entry: &AgentEntry) -> bool {
    matches!(entry.status, AgentStatus::Installed { .. })
}

/// 그 에이전트와 버전이 제공하는 훅 이벤트 이름.
///
/// - Claude Code: 31개.
/// - Codex: 0.169.0 미만이거나 버전을 해석하지 못하면 11개, 0.169.0 이상이면 `PreModelRequest`,
///   `PostModelRequest`, `Interrupt`를 더한 14개.
pub fn hook_events(agent: AgentKind, version: &str) -> Vec<&'static str> {
    match agent {
        AgentKind::Claude => CLAUDE_EVENTS.to_vec(),
        AgentKind::Codex if version_at_least(version, (0, 169, 0)) => CODEX_BASE_EVENTS
            .iter()
            .chain(CODEX_0_169_EVENTS.iter())
            .copied()
            .collect(),
        AgentKind::Codex => CODEX_BASE_EVENTS.to_vec(),
    }
}

/// `major.minor.patch`로 시작하는 버전이 `min` 이상인지. 해석하지 못하면 false다.
fn version_at_least(version: &str, min: (u64, u64, u64)) -> bool {
    let mut parts = version
        .trim()
        .trim_start_matches('v')
        .split(|c: char| !c.is_ascii_digit())
        .map(str::parse::<u64>);
    match (parts.next(), parts.next(), parts.next()) {
        (Some(Ok(major)), Some(Ok(minor)), Some(Ok(patch))) => (major, minor, patch) >= min,
        _ => false,
    }
}

/// 이벤트 하나에 등록할 훅 handler(JSON 객체). `matcher`는 넣지 않아 모든 도구에 실행된다.
///
/// - Posix: `{"type":"command","command":"<sh 한 줄>","timeout":t}`
/// - Windows + Codex: `{"type":"command","command":"<cmd 한 줄>","timeout":t}`. Codex가 `cmd.exe /C`로 실행한다.
/// - Windows + Claude Code: `{"type":"command","command":"cmd.exe","args":["/d","/c","<cmd 한 줄>"],"timeout":t}`.
///   Git Bash가 있든 없든 셸을 거치지 않고 `cmd.exe`를 바로 실행한다.
///
/// 어느 쪽이든 스크립트가 없으면 입력만 읽고 성공으로 끝난다. `t`는 `SessionEnd`만 3, 나머지는 5(초)다.
pub fn hook_handler(target: &InstallTarget, shell: ShellFamily, event: &str) -> Value {
    let agent = target.agent.command();
    let timeout = if event == "SessionEnd" {
        SESSION_END_TIMEOUT
    } else {
        DEFAULT_TIMEOUT
    };
    match (shell, target.agent) {
        (ShellFamily::Posix, _) => {
            json!({"type": "command", "command": posix_line(target, agent, event), "timeout": timeout})
        }
        (ShellFamily::Windows, AgentKind::Codex) => {
            json!({"type": "command", "command": windows_line(target, agent, event), "timeout": timeout})
        }
        (ShellFamily::Windows, AgentKind::Claude) => json!({
            "type": "command",
            "command": "cmd.exe",
            "args": ["/d", "/c", windows_line(target, agent, event)],
            "timeout": timeout
        }),
    }
}

/// sh 훅 명령 한 줄. 맨 앞의 `:` 명령은 아무것도 하지 않고 앱 항목 표시만 남긴다.
fn posix_line(target: &InstallTarget, agent: &str, event: &str) -> String {
    let script = format!("{}/.namaste-world/hooks/namaste-hook.sh", target.home);
    format!(
        r#": {HOOK_MARKER}; f="{script}"; if [ -f "$f" ]; then /bin/sh "$f" {agent} {event} "{endpoint}" >/dev/null 2>&1; else cat >/dev/null 2>&1; fi; exit 0"#,
        endpoint = target.endpoint_path
    )
}

/// cmd 훅 명령 한 줄. 표시는 스크립트가 무시하는 세 번째 인자로 넣는다.
fn windows_line(target: &InstallTarget, agent: &str, event: &str) -> String {
    let script = format!(r"{}\.namaste-world\hooks\namaste-hook.cmd", target.home);
    format!(r#"if exist "{script}" ("{script}" {agent} {event} {HOOK_MARKER}) else (more >nul)"#)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::domain::model::{AgentStatus, Environment, FailureReason};
    use serde_json::json;

    const CLAUDE_EVENTS: [&str; 31] = [
        "SessionStart",
        "Setup",
        "UserPromptSubmit",
        "UserPromptExpansion",
        "PreToolUse",
        "PermissionRequest",
        "PermissionDenied",
        "PostToolUse",
        "PostToolUseFailure",
        "PostToolBatch",
        "Notification",
        "MessageDisplay",
        "SubagentStart",
        "SubagentStop",
        "TaskCreated",
        "TaskCompleted",
        "Stop",
        "StopFailure",
        "TeammateIdle",
        "InstructionsLoaded",
        "ConfigChange",
        "CwdChanged",
        "DirectoryAdded",
        "FileChanged",
        "PreCompact",
        "PostCompact",
        "PreModelSwitch",
        "PostModelSwitch",
        "Elicitation",
        "ElicitationResult",
        "SessionEnd",
    ];

    const CODEX_BASE_EVENTS: [&str; 11] = [
        "PreToolUse",
        "PermissionRequest",
        "PostToolUse",
        "PreCompact",
        "PostCompact",
        "SessionStart",
        "SessionEnd",
        "UserPromptSubmit",
        "SubagentStart",
        "SubagentStop",
        "Stop",
    ];

    fn sorted(mut v: Vec<&'static str>) -> Vec<&'static str> {
        v.sort_unstable();
        v
    }

    fn codex_extended() -> Vec<&'static str> {
        let mut v = CODEX_BASE_EVENTS.to_vec();
        v.extend(["PreModelRequest", "PostModelRequest", "Interrupt"]);
        sorted(v)
    }

    fn entry(status: AgentStatus) -> AgentEntry {
        AgentEntry {
            agent: AgentKind::Codex,
            environment: Environment::Wsl {
                distro: "Ubuntu".into(),
            },
            status,
        }
    }

    fn mac_target(agent: AgentKind) -> InstallTarget {
        InstallTarget {
            agent,
            environment: Environment::Macos,
            version: "1.0.0".into(),
            home: "/Users/a".into(),
            endpoint_path: "/Users/a/.namaste-world/endpoint".into(),
        }
    }

    fn windows_target(agent: AgentKind) -> InstallTarget {
        InstallTarget {
            agent,
            environment: Environment::Windows,
            version: "1.0.0".into(),
            home: r"C:\Users\홍 길동".into(),
            endpoint_path: r"C:\Users\홍 길동\.namaste-world\endpoint".into(),
        }
    }

    fn posix_line(agent: &str, event: &str) -> String {
        format!(
            r#": namaste-world-hook; f="/Users/a/.namaste-world/hooks/namaste-hook.sh"; if [ -f "$f" ]; then /bin/sh "$f" {agent} {event} "/Users/a/.namaste-world/endpoint" >/dev/null 2>&1; else cat >/dev/null 2>&1; fi; exit 0"#
        )
    }

    fn windows_line(agent: &str, event: &str) -> String {
        let script = r"C:\Users\홍 길동\.namaste-world\hooks\namaste-hook.cmd";
        format!(
            r#"if exist "{script}" ("{script}" {agent} {event} namaste-world-hook) else (more >nul)"#
        )
    }

    #[test]
    fn fr_hook_002_installs_only_installed_entries() {
        assert!(should_install(&entry(AgentStatus::Installed {
            path: "/usr/bin/codex".into(),
            version: "0.146.0".into(),
        })));
    }

    #[test]
    fn fr_hook_002_skips_not_installed_failed_and_stopped_entries() {
        assert!(!should_install(&entry(AgentStatus::NotInstalled)));
        assert!(!should_install(&entry(AgentStatus::Failed {
            reason: FailureReason::Timeout,
        })));
        assert!(!should_install(&entry(AgentStatus::Stopped)));
    }

    #[test]
    fn fr_hook_003_claude_registers_all_31_events() {
        let events = hook_events(AgentKind::Claude, "2.1.325");
        assert_eq!(events.len(), 31);
        assert_eq!(sorted(events), sorted(CLAUDE_EVENTS.to_vec()));
    }

    #[test]
    fn fr_hook_003_claude_skips_events_that_replace_default_behavior() {
        let events = hook_events(AgentKind::Claude, "2.1.325");
        assert!(!events.contains(&"WorktreeCreate"), "{events:?}");
        assert!(!events.contains(&"WorktreeRemove"), "{events:?}");
    }

    #[test]
    fn fr_hook_003_codex_below_0_169_registers_11_events() {
        for version in ["0.146.0", "0.168.9"] {
            let events = hook_events(AgentKind::Codex, version);
            assert_eq!(events.len(), 11, "{version}");
            assert_eq!(
                sorted(events),
                sorted(CODEX_BASE_EVENTS.to_vec()),
                "{version}"
            );
        }
    }

    #[test]
    fn fr_hook_003_codex_from_0_169_registers_14_events() {
        for version in ["0.169.0", "0.170.2", "1.0.0"] {
            let events = hook_events(AgentKind::Codex, version);
            assert_eq!(events.len(), 14, "{version}");
            assert_eq!(sorted(events), codex_extended(), "{version}");
        }
    }

    #[test]
    fn fr_hook_003_codex_unreadable_version_registers_11_events() {
        for version in ["", "unknown", "v-next"] {
            assert_eq!(
                sorted(hook_events(AgentKind::Codex, version)),
                sorted(CODEX_BASE_EVENTS.to_vec()),
                "{version:?}"
            );
        }
    }

    #[test]
    fn fr_hook_003_posix_handler_runs_script_for_all_tools() {
        assert_eq!(
            hook_handler(
                &mac_target(AgentKind::Claude),
                ShellFamily::Posix,
                "PreToolUse"
            ),
            json!({"type": "command", "command": posix_line("claude", "PreToolUse"), "timeout": 5})
        );
        assert_eq!(
            hook_handler(&mac_target(AgentKind::Codex), ShellFamily::Posix, "Stop"),
            json!({"type": "command", "command": posix_line("codex", "Stop"), "timeout": 5})
        );
    }

    #[test]
    fn fr_hook_011_windows_codex_handler_is_cmd_line() {
        assert_eq!(
            hook_handler(
                &windows_target(AgentKind::Codex),
                ShellFamily::Windows,
                "UserPromptSubmit"
            ),
            json!({
                "type": "command",
                "command": windows_line("codex", "UserPromptSubmit"),
                "timeout": 5
            })
        );
    }

    #[test]
    fn fr_hook_011_windows_claude_handler_execs_cmd_exe() {
        assert_eq!(
            hook_handler(
                &windows_target(AgentKind::Claude),
                ShellFamily::Windows,
                "Stop"
            ),
            json!({
                "type": "command",
                "command": "cmd.exe",
                "args": ["/d", "/c", windows_line("claude", "Stop")],
                "timeout": 5
            })
        );
    }

    #[test]
    fn sc_hook_001_session_end_timeout_is_3_seconds() {
        let cases = [
            (mac_target(AgentKind::Claude), ShellFamily::Posix),
            (mac_target(AgentKind::Codex), ShellFamily::Posix),
            (windows_target(AgentKind::Claude), ShellFamily::Windows),
            (windows_target(AgentKind::Codex), ShellFamily::Windows),
        ];
        for (target, shell) in cases {
            assert_eq!(
                hook_handler(&target, shell, "SessionEnd")["timeout"],
                json!(3),
                "{target:?}"
            );
        }
    }

    #[test]
    fn fr_hook_004_every_handler_carries_marker_and_no_matcher() {
        let cases = [
            (mac_target(AgentKind::Claude), ShellFamily::Posix),
            (mac_target(AgentKind::Codex), ShellFamily::Posix),
            (windows_target(AgentKind::Claude), ShellFamily::Windows),
            (windows_target(AgentKind::Codex), ShellFamily::Windows),
        ];
        for (target, shell) in cases {
            for event in hook_events(target.agent, &target.version) {
                let handler = hook_handler(&target, shell, event);
                assert!(handler.get("matcher").is_none(), "{event}");
                assert!(
                    handler.to_string().contains(HOOK_MARKER),
                    "{target:?} {event}"
                );
                let expected_timeout = if event == "SessionEnd" { 3 } else { 5 };
                assert_eq!(handler["timeout"], json!(expected_timeout), "{event}");
            }
        }
    }
}
