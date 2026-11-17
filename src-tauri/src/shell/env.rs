use super::model::{ShellEnv, ShellEnvFailure, ShellKind};
use crate::agent::domain::model::AgentKind;
use std::collections::BTreeMap;
use std::time::Duration;

/// `printf '\0<m>\0'; env -0; printf '\0<m>\0'` 출력에서 환경을 꺼낸다. 마커 앞뒤의 셸 설정 출력은 무시한다.
/// 마커가 두 번 없거나 `KEY=VALUE`가 아닌 항목만 있으면 None. `=` 없는 항목은 버린다.
pub fn parse_env_nul(stdout: &[u8], marker: &str) -> Option<BTreeMap<String, String>> {
    let fence = format!("\0{marker}\0").into_bytes();
    let start = find(stdout, &fence, 0)? + fence.len();
    let end = find(stdout, &fence, start)?;
    let vars: BTreeMap<String, String> = stdout[start..end]
        .split(|byte| *byte == 0)
        .filter_map(|item| key_value(&String::from_utf8_lossy(item)))
        .collect();
    (!vars.is_empty()).then_some(vars)
}

/// `haystack[from..]`에서 `needle`이 처음 나오는 위치.
fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|index| from + index)
}

/// `KEY=VALUE` 한 항목. 이름이 비어 있으면(Windows의 `=C:=C:\` 같은 항목) 버린다.
fn key_value(item: &str) -> Option<(String, String)> {
    item.split_once('=')
        .filter(|(key, _)| !key.is_empty())
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
}

/// `<m>{"KEY":"VALUE",...}<m>` 형태의 PowerShell 출력에서 환경을 꺼낸다. JSON이 아니거나 값이 문자열이 아니면 None.
pub fn parse_env_json(stdout: &str, marker: &str) -> Option<BTreeMap<String, String>> {
    let (_, rest) = stdout.split_once(marker)?;
    let (json, _) = rest.split_once(marker)?;
    serde_json::from_str(json).ok()
}

/// 앱이 받은 환경 위에 셸 환경을 덮는다. 앱에만 있는 변수는 남는다(FR-AGENT-012).
/// `case_insensitive`(Windows)면 이름을 대소문자 구분 없이 같은 변수로 보고 셸 쪽 이름을 쓴다.
pub fn merge_env(
    app: &BTreeMap<String, String>,
    shell: &BTreeMap<String, String>,
    case_insensitive: bool,
) -> ShellEnv {
    let overridden = |key: &str| {
        if case_insensitive {
            shell.keys().any(|name| name.eq_ignore_ascii_case(key))
        } else {
            shell.contains_key(key)
        }
    };
    let vars = app
        .iter()
        .filter(|(key, _)| !overridden(key))
        .chain(shell)
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    ShellEnv::new(vars)
}

/// 에이전트 설정 폴더. Claude는 `CLAUDE_CONFIG_DIR`, Codex는 `CODEX_HOME`이 비어 있지 않으면 그 값,
/// 아니면 `home` + `separator` + `.claude`/`.codex`(FR-HOOK-002).
pub fn config_dir(env: &ShellEnv, agent: AgentKind, home: &str, separator: char) -> String {
    let (variable, folder) = match agent {
        AgentKind::Claude => ("CLAUDE_CONFIG_DIR", ".claude"),
        AgentKind::Codex => ("CODEX_HOME", ".codex"),
    };
    env.get(variable)
        .filter(|dir| !dir.is_empty())
        .map_or_else(|| format!("{home}{separator}{folder}"), str::to_owned)
}

/// 셸을 한 번 실행한 결과. `platform/`이 `run_checked` 결과에서 만든다.
/// `stdout`에 환경 변수 값이 들어 있으므로 `Debug`는 길이만 찍는다(FR-AGENT-017).
#[derive(Clone, PartialEq, Eq)]
pub enum ShellRun {
    LaunchFailed,
    Timeout,
    Exited { success: bool, stdout: Vec<u8> },
}

impl std::fmt::Debug for ShellRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellRun::LaunchFailed => write!(f, "LaunchFailed"),
            ShellRun::Timeout => write!(f, "Timeout"),
            ShellRun::Exited { success, stdout } => {
                write!(
                    f,
                    "Exited {{ success: {success}, stdout: {} bytes }}",
                    stdout.len()
                )
            }
        }
    }
}

/// 셸 실행 결과를 CLI 실행 환경과 실패 이유로 바꾼다(FR-AGENT-016, FR-AGENT-018).
/// 띄우기 실패 → LaunchFailed, 시간 초과 → Timeout, 0이 아닌 종료 → ExitedWithError, `parse`가 None → UnreadableOutput.
/// 실패하면 앱 환경만 담은 `ShellEnv`를 준다. 성공하면 `merge_env(app, 읽은 환경, case_insensitive)`다.
pub fn shell_env_outcome(
    app: &BTreeMap<String, String>,
    run: ShellRun,
    parse: impl Fn(&[u8]) -> Option<BTreeMap<String, String>>,
    case_insensitive: bool,
) -> (ShellEnv, Option<ShellEnvFailure>) {
    let read = match run {
        ShellRun::LaunchFailed => Err(ShellEnvFailure::LaunchFailed),
        ShellRun::Timeout => Err(ShellEnvFailure::Timeout),
        ShellRun::Exited { success: false, .. } => Err(ShellEnvFailure::ExitedWithError),
        ShellRun::Exited {
            success: true,
            stdout,
        } => parse(&stdout).ok_or(ShellEnvFailure::UnreadableOutput),
    };
    match read {
        Ok(shell) => (merge_env(app, &shell, case_insensitive), None),
        Err(failure) => (ShellEnv::new(app.clone()), Some(failure)),
    }
}

/// Windows에서 이 셸의 환경을 셸을 띄워 읽는가. 명령 프롬프트는 앱 환경을 그대로 써서 false(FR-AGENT-014).
pub fn reads_shell(shell: ShellKind) -> bool {
    match shell {
        ShellKind::PowerShell | ShellKind::GitBash => true,
        ShellKind::Cmd => false,
    }
}

/// 셸 환경을 읽은 뒤 감지에 남은 시간. `total - elapsed`이고 0 아래로 내려가지 않는다(SC-AGENT-001, SC-AGENT-003).
pub fn remaining_limit(total: Duration, elapsed: Duration) -> Duration {
    total.saturating_sub(elapsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const MARKER: &str = "__NW_ENV_MARK__";

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// `printf '\0<m>\0'; env -0; printf '\0<m>\0'`가 찍는 모양. 각 항목은 NUL로 끝난다.
    fn nul_output(prefix: &str, entries: &[String], suffix: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(prefix.as_bytes());
        out.extend_from_slice(format!("\0{MARKER}\0").as_bytes());
        for entry in entries {
            out.extend_from_slice(entry.as_bytes());
            out.push(0);
        }
        out.extend_from_slice(format!("\0{MARKER}\0").as_bytes());
        out.extend_from_slice(suffix.as_bytes());
        out
    }

    fn entries(pairs: &[(&str, &str)]) -> Vec<String> {
        pairs.iter().map(|(k, v)| format!("{k}={v}")).collect()
    }

    // parse_env_nul

    #[test]
    fn fr_agent_012_parse_env_nul_reads_all_variables_between_markers() {
        let out = nul_output(
            "",
            &entries(&[
                ("CLAUDE_CONFIG_DIR", "/Users/a/work-claude"),
                ("ANTHROPIC_BASE_URL", "https://proxy.example"),
                ("PATH", "/opt/homebrew/bin:/usr/bin"),
            ]),
            "",
        );

        assert_eq!(
            parse_env_nul(&out, MARKER),
            Some(map(&[
                ("CLAUDE_CONFIG_DIR", "/Users/a/work-claude"),
                ("ANTHROPIC_BASE_URL", "https://proxy.example"),
                ("PATH", "/opt/homebrew/bin:/usr/bin"),
            ]))
        );
    }

    #[test]
    fn fr_agent_012_parse_env_nul_ignores_shell_config_output_around_markers() {
        let out = nul_output(
            "Welcome!\nNOISE=before\n",
            &entries(&[("OPENAI_API_KEY", "sk-test")]),
            "bye\nNOISE=after\n",
        );

        assert_eq!(
            parse_env_nul(&out, MARKER),
            Some(map(&[("OPENAI_API_KEY", "sk-test")]))
        );
    }

    #[test]
    fn fr_agent_012_parse_env_nul_keeps_newlines_in_values() {
        let out = nul_output("", &entries(&[("MULTI", "line1\nline2")]), "");

        assert_eq!(
            parse_env_nul(&out, MARKER),
            Some(map(&[("MULTI", "line1\nline2")]))
        );
    }

    #[test]
    fn fr_agent_012_parse_env_nul_drops_items_without_equals() {
        let out = nul_output(
            "",
            &["A=1".to_string(), "garbage".to_string(), "B=2".to_string()],
            "",
        );

        assert_eq!(
            parse_env_nul(&out, MARKER),
            Some(map(&[("A", "1"), ("B", "2")]))
        );
    }

    #[test]
    fn fr_agent_016_parse_env_nul_without_markers_is_none() {
        let out = b"A=1\0B=2\0".to_vec();

        assert_eq!(parse_env_nul(&out, MARKER), None);
    }

    #[test]
    fn fr_agent_016_parse_env_nul_with_only_one_marker_is_none() {
        let mut out = format!("\0{MARKER}\0").into_bytes();
        out.extend_from_slice(b"A=1\0B=2\0");

        assert_eq!(parse_env_nul(&out, MARKER), None);
    }

    #[test]
    fn fr_agent_016_parse_env_nul_with_no_key_value_items_is_none() {
        let out = nul_output("", &["garbage".to_string(), "more".to_string()], "");

        assert_eq!(parse_env_nul(&out, MARKER), None);
    }

    proptest! {
        #[test]
        fn fr_agent_012_parse_env_nul_round_trips_any_environment(
            vars in prop::collection::btree_map("[A-Z_][A-Z0-9_]{0,10}", "[^\u{0}]{0,20}", 1..8),
            prefix in "[a-z =\n]{0,30}",
            suffix in "[a-z =\n]{0,30}",
        ) {
            let items: Vec<String> = vars.iter().map(|(k, v)| format!("{k}={v}")).collect();
            let out = nul_output(&prefix, &items, &suffix);

            prop_assert_eq!(parse_env_nul(&out, MARKER), Some(vars));
        }
    }

    // parse_env_json

    #[test]
    fn fr_agent_014_parse_env_json_reads_powershell_output() {
        let out = format!(
            "profile loaded\r\n{MARKER}{{\"ANTHROPIC_API_KEY\":\"sk-ant\",\"Path\":\"C:\\\\Program Files\\\\nodejs;C:\\\\Windows\"}}{MARKER}\r\n"
        );

        assert_eq!(
            parse_env_json(&out, MARKER),
            Some(map(&[
                ("ANTHROPIC_API_KEY", "sk-ant"),
                ("Path", "C:\\Program Files\\nodejs;C:\\Windows"),
            ]))
        );
    }

    #[test]
    fn fr_agent_016_parse_env_json_that_is_not_json_is_none() {
        let out = format!("{MARKER}not json{MARKER}");

        assert_eq!(parse_env_json(&out, MARKER), None);
    }

    #[test]
    fn fr_agent_016_parse_env_json_with_non_string_value_is_none() {
        let number = format!("{MARKER}{{\"A\":\"1\",\"B\":2}}{MARKER}");
        let null = format!("{MARKER}{{\"A\":null}}{MARKER}");

        assert_eq!(parse_env_json(&number, MARKER), None);
        assert_eq!(parse_env_json(&null, MARKER), None);
    }

    #[test]
    fn fr_agent_016_parse_env_json_without_markers_is_none() {
        assert_eq!(parse_env_json("{\"A\":\"1\"}", MARKER), None);
        assert_eq!(
            parse_env_json(&format!("{MARKER}{{\"A\":\"1\"}}"), MARKER),
            None
        );
    }

    proptest! {
        #[test]
        fn fr_agent_014_parse_env_json_round_trips_any_environment(
            vars in prop::collection::btree_map("[A-Za-z_][A-Za-z0-9_()]{0,10}", "[A-Za-z0-9 =;:/\\\\\"\n]{0,20}", 1..8),
            prefix in "[a-z {}\r\n]{0,30}",
            suffix in "[a-z {}\r\n]{0,30}",
        ) {
            let json = serde_json::to_string(&vars).unwrap();
            let out = format!("{prefix}{MARKER}{json}{MARKER}{suffix}");

            prop_assert_eq!(parse_env_json(&out, MARKER), Some(vars));
        }
    }

    // merge_env

    #[test]
    fn fr_agent_012_merge_env_overlays_shell_and_keeps_app_only_vars() {
        let app = map(&[("PATH", "/usr/bin"), ("__CFBundleIdentifier", "app")]);
        let shell = map(&[
            ("PATH", "/opt/homebrew/bin:/usr/bin"),
            ("CLAUDE_CONFIG_DIR", "/Users/a/work-claude"),
        ]);

        let env = merge_env(&app, &shell, false);

        assert_eq!(env.get("PATH"), Some("/opt/homebrew/bin:/usr/bin"));
        assert_eq!(env.get("CLAUDE_CONFIG_DIR"), Some("/Users/a/work-claude"));
        assert_eq!(env.get("__CFBundleIdentifier"), Some("app"));
        assert_eq!(env.vars().count(), 3);
    }

    #[test]
    fn fr_agent_012_merge_env_case_sensitive_keeps_both_spellings() {
        let app = map(&[("Path", "a")]);
        let shell = map(&[("PATH", "b")]);

        let env = merge_env(&app, &shell, false);

        assert_eq!(env.get("Path"), Some("a"));
        assert_eq!(env.get("PATH"), Some("b"));
    }

    #[test]
    fn fr_agent_014_merge_env_case_insensitive_uses_shell_name_and_value() {
        let app = map(&[("Path", "C:\\Windows"), ("USERPROFILE", "C:\\Users\\a")]);
        let shell = map(&[
            ("PATH", "C:\\Git\\bin;C:\\Windows"),
            ("ANTHROPIC_API_KEY", "k"),
        ]);

        let env = merge_env(&app, &shell, true);

        assert_eq!(env.get("PATH"), Some("C:\\Git\\bin;C:\\Windows"));
        assert_eq!(env.get("Path"), None);
        assert_eq!(env.get("USERPROFILE"), Some("C:\\Users\\a"));
        assert_eq!(env.get("ANTHROPIC_API_KEY"), Some("k"));
        assert_eq!(env.vars().count(), 3);
    }

    #[test]
    fn fr_agent_016_merge_env_with_empty_shell_is_app_env() {
        let app = map(&[("PATH", "/usr/bin"), ("HOME", "/Users/a")]);

        let env = merge_env(&app, &BTreeMap::new(), false);

        assert_eq!(env, ShellEnv::new(app));
    }

    proptest! {
        #[test]
        fn fr_agent_012_merge_env_is_shell_over_app(
            app in prop::collection::btree_map("[A-Za-z_]{1,4}", "[a-z]{0,5}", 0..8),
            shell in prop::collection::btree_map("[A-Za-z_]{1,4}", "[a-z]{0,5}", 0..8),
        ) {
            let env = merge_env(&app, &shell, false);

            for (k, v) in &shell {
                prop_assert_eq!(env.get(k), Some(v.as_str()));
            }
            for (k, v) in &app {
                if !shell.contains_key(k) {
                    prop_assert_eq!(env.get(k), Some(v.as_str()));
                }
            }
            let union = app.keys().chain(shell.keys()).collect::<std::collections::BTreeSet<_>>();
            prop_assert_eq!(env.vars().count(), union.len());
        }

        #[test]
        fn fr_agent_014_merge_env_case_insensitive_has_one_name_per_shell_var(
            app in prop::collection::btree_map("[A-Za-z]{1,3}", "[a-z]{0,5}", 0..8),
            shell in prop::collection::btree_map("[A-Z]{1,3}", "[a-z]{0,5}", 0..8),
        ) {
            let env = merge_env(&app, &shell, true);

            for (k, v) in &shell {
                prop_assert_eq!(env.get(k), Some(v.as_str()));
                let same_name = env
                    .vars()
                    .filter(|(name, _)| name.eq_ignore_ascii_case(k))
                    .count();
                prop_assert_eq!(same_name, 1);
            }
            for (k, v) in &app {
                if !shell.keys().any(|s| s.eq_ignore_ascii_case(k)) {
                    prop_assert_eq!(env.get(k), Some(v.as_str()));
                }
            }
        }
    }

    // config_dir

    fn env_of(pairs: &[(&str, &str)]) -> ShellEnv {
        ShellEnv::new(map(pairs))
    }

    #[test]
    fn fr_hook_002_config_dir_uses_claude_config_dir() {
        let env = env_of(&[("CLAUDE_CONFIG_DIR", "/Users/a/work-claude")]);

        assert_eq!(
            config_dir(&env, AgentKind::Claude, "/Users/a", '/'),
            "/Users/a/work-claude"
        );
    }

    #[test]
    fn fr_hook_002_config_dir_uses_codex_home() {
        let env = env_of(&[("CODEX_HOME", "/home/a/codex-work")]);

        assert_eq!(
            config_dir(&env, AgentKind::Codex, "/home/a", '/'),
            "/home/a/codex-work"
        );
    }

    #[test]
    fn fr_hook_002_config_dir_without_variable_is_home_dot_folder() {
        let env = env_of(&[]);

        assert_eq!(
            config_dir(&env, AgentKind::Claude, "/Users/a", '/'),
            "/Users/a/.claude"
        );
        assert_eq!(
            config_dir(&env, AgentKind::Codex, "/Users/a", '/'),
            "/Users/a/.codex"
        );
    }

    #[test]
    fn fr_hook_002_config_dir_windows_without_codex_home_is_userprofile_codex() {
        let env = env_of(&[("Path", "C:\\Windows")]);

        assert_eq!(
            config_dir(&env, AgentKind::Codex, "C:\\Users\\a", '\\'),
            "C:\\Users\\a\\.codex"
        );
    }

    #[test]
    fn fr_hook_002_config_dir_empty_variable_is_home_dot_folder() {
        let env = env_of(&[("CLAUDE_CONFIG_DIR", ""), ("CODEX_HOME", "")]);

        assert_eq!(
            config_dir(&env, AgentKind::Claude, "/Users/a", '/'),
            "/Users/a/.claude"
        );
        assert_eq!(
            config_dir(&env, AgentKind::Codex, "/Users/a", '/'),
            "/Users/a/.codex"
        );
    }

    #[test]
    fn fr_hook_002_config_dir_reads_only_its_own_agent_variable() {
        let env = env_of(&[
            ("CLAUDE_CONFIG_DIR", "/Users/a/work-claude"),
            ("CODEX_HOME", "/Users/a/codex-work"),
        ]);

        assert_eq!(
            config_dir(&env, AgentKind::Claude, "/Users/a", '/'),
            "/Users/a/work-claude"
        );
        assert_eq!(
            config_dir(&env, AgentKind::Codex, "/Users/a", '/'),
            "/Users/a/codex-work"
        );
    }

    // shell_env_outcome

    fn app_env() -> BTreeMap<String, String> {
        map(&[("HOME", "/Users/a"), ("PATH", "/usr/bin:/bin")])
    }

    /// 넘겨받은 출력이 `expected`일 때만 `parsed`를 주는 가짜 해석기.
    fn parse_only(
        expected: &'static [u8],
        parsed: BTreeMap<String, String>,
    ) -> impl Fn(&[u8]) -> Option<BTreeMap<String, String>> {
        move |stdout| (stdout == expected).then(|| parsed.clone())
    }

    #[test]
    fn fr_agent_016_launch_failed_uses_app_env_with_launch_failed_reason() {
        let (env, failure) = shell_env_outcome(&app_env(), ShellRun::LaunchFailed, |_| None, false);

        assert_eq!(env, ShellEnv::new(app_env()));
        assert_eq!(failure, Some(ShellEnvFailure::LaunchFailed));
    }

    #[test]
    fn sc_agent_003_timeout_uses_app_env_with_timeout_reason() {
        let (env, failure) = shell_env_outcome(&app_env(), ShellRun::Timeout, |_| None, false);

        assert_eq!(env, ShellEnv::new(app_env()));
        assert_eq!(failure, Some(ShellEnvFailure::Timeout));
    }

    #[test]
    fn fr_agent_016_scenario_1_shell_exiting_with_error_uses_app_env() {
        // 셸 설정 파일이 오류로 끝나면 출력이 읽을 만해도 쓰지 않는다
        let run = ShellRun::Exited {
            success: false,
            stdout: b"partial".to_vec(),
        };
        let parse = parse_only(b"partial", map(&[("CLAUDE_CONFIG_DIR", "/x")]));

        let (env, failure) = shell_env_outcome(&app_env(), run, parse, false);

        assert_eq!(env, ShellEnv::new(app_env()));
        assert_eq!(failure, Some(ShellEnvFailure::ExitedWithError));
    }

    #[test]
    fn fr_agent_016_unreadable_output_uses_app_env() {
        let run = ShellRun::Exited {
            success: true,
            stdout: b"garbage".to_vec(),
        };

        let (env, failure) = shell_env_outcome(&app_env(), run, |_| None, false);

        assert_eq!(env, ShellEnv::new(app_env()));
        assert_eq!(failure, Some(ShellEnvFailure::UnreadableOutput));
    }

    #[test]
    fn fr_agent_012_read_shell_env_is_merged_over_app_env_without_failure() {
        let run = ShellRun::Exited {
            success: true,
            stdout: b"env-output".to_vec(),
        };
        let parse = parse_only(
            b"env-output",
            map(&[
                ("PATH", "/opt/homebrew/bin:/usr/bin:/bin"),
                ("ANTHROPIC_BASE_URL", "https://proxy.example"),
            ]),
        );

        let (env, failure) = shell_env_outcome(&app_env(), run, parse, false);

        assert_eq!(
            env,
            ShellEnv::new(map(&[
                ("ANTHROPIC_BASE_URL", "https://proxy.example"),
                ("HOME", "/Users/a"),
                ("PATH", "/opt/homebrew/bin:/usr/bin:/bin"),
            ]))
        );
        assert_eq!(failure, None);
    }

    #[test]
    fn fr_agent_014_read_windows_env_merges_names_case_insensitively() {
        let app = map(&[("Path", "C:\\Windows"), ("USERPROFILE", "C:\\Users\\a")]);
        let run = ShellRun::Exited {
            success: true,
            stdout: b"json".to_vec(),
        };
        let parse = parse_only(b"json", map(&[("PATH", "C:\\Tools;C:\\Windows")]));

        let (env, failure) = shell_env_outcome(&app, run, parse, true);

        assert_eq!(
            env,
            ShellEnv::new(map(&[
                ("PATH", "C:\\Tools;C:\\Windows"),
                ("USERPROFILE", "C:\\Users\\a"),
            ]))
        );
        assert_eq!(failure, None);
    }

    fn failing_run() -> impl Strategy<Value = ShellRun> {
        prop_oneof![
            Just(ShellRun::LaunchFailed),
            Just(ShellRun::Timeout),
            prop::collection::vec(any::<u8>(), 0..16).prop_map(|stdout| ShellRun::Exited {
                success: false,
                stdout
            }),
        ]
    }

    proptest! {
        #[test]
        fn fr_agent_016_any_failure_gives_exactly_app_env_and_a_reason(
            app in prop::collection::btree_map("[A-Z_]{1,8}", "[a-z/:]{0,8}", 0..6),
            shell in prop::collection::btree_map("[A-Z_]{1,8}", "[a-z/:]{0,8}", 0..6),
            run in failing_run(),
            case_insensitive in any::<bool>(),
        ) {
            let parse = move |_: &[u8]| Some(shell.clone());

            let (env, failure) = shell_env_outcome(&app, run, parse, case_insensitive);

            prop_assert_eq!(env, ShellEnv::new(app.clone()));
            prop_assert!(failure.is_some());
        }
    }

    // reads_shell

    #[test]
    fn fr_agent_014_powershell_and_git_bash_read_their_config() {
        assert!(reads_shell(ShellKind::PowerShell));
        assert!(reads_shell(ShellKind::GitBash));
    }

    #[test]
    fn fr_agent_014_scenario_3_cmd_does_not_launch_a_shell() {
        assert!(!reads_shell(ShellKind::Cmd));
    }

    // remaining_limit

    #[test]
    fn sc_agent_003_time_spent_reading_shell_is_taken_from_detection_limit() {
        assert_eq!(
            remaining_limit(Duration::from_secs(10), Duration::from_secs(3)),
            Duration::from_secs(7)
        );
    }

    #[test]
    fn sc_agent_003_remaining_limit_does_not_go_below_zero() {
        assert_eq!(
            remaining_limit(Duration::from_secs(10), Duration::from_secs(10)),
            Duration::ZERO
        );
        assert_eq!(
            remaining_limit(Duration::from_secs(10), Duration::from_secs(12)),
            Duration::ZERO
        );
    }

    proptest! {
        #[test]
        fn sc_agent_001_shell_read_plus_detection_stays_within_total(
            total_ms in 0u64..20_000,
            elapsed_ms in 0u64..20_000,
        ) {
            let total = Duration::from_millis(total_ms);
            let elapsed = Duration::from_millis(elapsed_ms);

            let remaining = remaining_limit(total, elapsed);

            if elapsed <= total {
                prop_assert_eq!(elapsed + remaining, total);
            } else {
                prop_assert_eq!(remaining, Duration::ZERO);
            }
        }
    }
}
