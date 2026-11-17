/// 배포판 안에서 `sh -c`로 돌릴 스크립트. `getent passwd`로 찾은 로그인 셸이 bash/zsh면 `-ilc`, 그 밖이거나 못 찾으면
/// `/bin/sh -lc`로 `inner`를 실행한다(FR-AGENT-013). `inner`는 stdout과 stderr에 `fence_marker`를 한 줄 찍은 뒤 실제 명령을 exec한다.
pub fn login_shell_script(inner: &str, fence_marker: &str) -> String {
    let fenced =
        format!("printf '%s\\n' '{fence_marker}'; printf '%s\\n' '{fence_marker}' >&2; {inner}");
    let quoted = shell_quote(&fenced);
    [
        "s=$(getent passwd \"$(id -un)\" 2>/dev/null | cut -d: -f7)",
        "case \"$s\" in */bash|*/zsh) [ -x \"$s\" ] || s='';; *) s='';; esac",
        &format!(
            "if [ -n \"$s\" ]; then exec \"$s\" -ilc {quoted}; else exec /bin/sh -lc {quoted}; fi"
        ),
    ]
    .join("; ")
}

/// `sh`의 작은따옴표 문자열. 안의 작은따옴표는 `'\''`로 바꾼다.
pub fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// 경계 마커 줄 뒤의 출력. 마커가 없으면 None(셸이 명령까지 가지 못함).
pub fn after_fence<'a>(output: &'a str, fence_marker: &str) -> Option<&'a str> {
    let line = format!("{fence_marker}\n");
    output
        .find(&line)
        .map(|index| &output[index + line.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const FENCE: &str = "__NW_FENCE__";

    #[test]
    fn fr_agent_013_after_fence_drops_shell_config_output() {
        let output = format!("Welcome to Ubuntu\nnvm loaded\n{FENCE}\n{{\"ok\":true}}\n");

        assert_eq!(after_fence(&output, FENCE), Some("{\"ok\":true}\n"));
    }

    #[test]
    fn fr_agent_013_after_fence_without_shell_output_before_it() {
        let output = format!("{FENCE}\ncodex-cli 0.50.0\n");

        assert_eq!(after_fence(&output, FENCE), Some("codex-cli 0.50.0\n"));
    }

    #[test]
    fn fr_agent_013_after_fence_without_marker_is_none() {
        assert_eq!(
            after_fence("bash: claude: command not found\n", FENCE),
            None
        );
        assert_eq!(after_fence("", FENCE), None);
    }

    proptest! {
        #[test]
        fn fr_agent_013_after_fence_returns_exactly_the_command_output(
            noise in prop::collection::vec("[a-z {}\":=]{0,12}", 0..4),
            body in "[a-z {}\":=\n]{0,40}",
        ) {
            let before: String = noise.iter().map(|line| format!("{line}\n")).collect();
            let output = format!("{before}{FENCE}\n{body}");

            prop_assert_eq!(after_fence(&output, FENCE), Some(body.as_str()));
        }

        #[test]
        fn fr_agent_013_after_fence_is_none_when_shell_never_reached_command(
            output in "[a-z {}\":=\n]{0,60}",
        ) {
            prop_assert_eq!(after_fence(&output, FENCE), None);
        }
    }

    // login_shell_script

    #[test]
    fn fr_agent_013_script_looks_up_login_shell_with_getent_passwd() {
        let script = login_shell_script("codex --version", FENCE);

        assert!(script.contains("getent passwd"), "{script}");
    }

    #[test]
    fn fr_agent_013_script_runs_bash_or_zsh_as_interactive_login_shell() {
        let script = login_shell_script("codex --version", FENCE);

        assert!(script.contains("-ilc"), "{script}");
    }

    #[test]
    fn fr_agent_013_scenario_3_script_falls_back_to_sh_login_shell() {
        let script = login_shell_script("codex --version", FENCE);

        assert!(script.contains("/bin/sh -lc"), "{script}");
    }

    /// 스크립트를 `/bin/sh -c`로 실제 실행한다. macOS에는 `getent`가 없어 `/bin/sh -lc` 경로를 탄다.
    #[cfg(unix)]
    fn run_script(inner: &str) -> std::process::Output {
        std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(login_shell_script(inner, FENCE))
            .stdin(std::process::Stdio::null())
            .output()
            .expect("/bin/sh를 실행하지 못함")
    }

    #[cfg(unix)]
    #[test]
    fn fr_agent_013_scenario_3_fallback_prints_fence_then_runs_command() {
        let output = run_script("printf '%s\\n' codex-cli");

        assert!(output.status.success(), "{output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(after_fence(&stdout, FENCE), Some("codex-cli\n"), "{stdout}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(after_fence(&stderr, FENCE).is_some(), "{stderr}");
    }

    #[cfg(unix)]
    #[test]
    fn fr_agent_013_scenario_3_fallback_keeps_single_quotes_in_command() {
        let output = run_script("printf '%s|%s\\n' 'a b' 'it'\"'\"'s'");

        assert!(output.status.success(), "{output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(after_fence(&stdout, FENCE), Some("a b|it's\n"), "{stdout}");
    }
}
