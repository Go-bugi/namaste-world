use super::model::{AgentKind, Found};
use std::collections::BTreeMap;

/// Docker Desktop 같은 도구가 쓰는 배포판. 사용자가 에이전트를 설치하는 곳이 아니라 확인하지 않는다.
const TOOL_DISTROS: [&str; 2] = ["docker-desktop", "docker-desktop-data"];

/// `wsl.exe -l -q` 출력에서 배포판 이름을 뽑는다.
/// 출력은 UTF-16LE(BOM이 있을 수 있음)이거나 UTF-8이다. 빈 줄과 NUL은 버리고,
/// 도구용 배포판(`docker-desktop`, `docker-desktop-data`)은 뺀다. 순서는 출력 순서 그대로다.
pub fn parse_distro_list(raw: &[u8]) -> Vec<String> {
    decode_wsl_output(raw)
        .lines()
        .map(|line| line.replace('\0', "").trim().to_owned())
        .filter(|name| !name.is_empty() && !TOOL_DISTROS.contains(&name.as_str()))
        .collect()
}

/// `wsl.exe`는 `WSL_UTF8`을 무시하는 버전에서 UTF-16LE로 출력한다. BOM이 있거나 두 번째 바이트가 NUL이면 UTF-16LE로 읽는다.
fn decode_wsl_output(raw: &[u8]) -> String {
    let utf16 = raw.starts_with(&[0xFF, 0xFE]) || raw.get(1) == Some(&0);
    if !utf16 {
        return String::from_utf8_lossy(raw).into_owned();
    }
    let body = raw.strip_prefix(&[0xFF, 0xFE]).unwrap_or(raw);
    let units: Vec<u16> = body
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

/// `--version` 출력에서 버전 문자열을 뽑는다. 첫 줄을 공백으로 나눈 토큰 가운데, 앞의 `v` 하나를 뗀 뒤
/// `숫자.숫자.숫자`로 시작하는 첫 토큰 전체다.
/// 예: "2.1.322 (Claude Code)" → Some("2.1.322"), "codex-cli v0.146.0" → Some("0.146.0"),
/// "2.1.322-beta.1" → Some("2.1.322-beta.1"), "2.1" → None, "" → None
pub fn parse_version(output: &str) -> Option<String> {
    output
        .lines()
        .next()?
        .split_whitespace()
        .map(|token| token.strip_prefix('v').unwrap_or(token))
        .find(|token| starts_with_three_numbers(token))
        .map(str::to_owned)
}

/// `숫자.숫자.숫자`로 시작하는가. 세 번째 숫자 뒤에는 무엇이 와도 된다.
fn starts_with_three_numbers(token: &str) -> bool {
    let mut parts = token.splitn(3, '.');
    let is_number = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    match (parts.next(), parts.next(), parts.next()) {
        (Some(major), Some(minor), Some(rest)) => {
            is_number(major) && is_number(minor) && rest.starts_with(|c: char| c.is_ascii_digit())
        }
        _ => false,
    }
}

/// 로그인 셸에서 `printf '<marker>%s<marker>' "$PATH"`로 찍은 출력에서 PATH를 뽑는다.
/// 셸 설정 파일이 찍은 다른 문구는 무시한다. 처음 두 마커 사이를 쓰고, 마커가 두 번 나오지 않으면 `None`이다.
pub fn parse_marked_path(stdout: &str, marker: &str) -> Option<String> {
    let (_, rest) = stdout.split_once(marker)?;
    let (path, _) = rest.split_once(marker)?;
    Some(path.to_owned())
}

/// WSL 프로브 스크립트 출력을 에이전트별 결과로 바꾼다.
/// 줄 형식은 `<marker>\t<agent>\t<path>\t<version 출력 첫 줄>`이다. 마커 없는 줄, 모르는 에이전트,
/// POSIX 절대 경로가 아닌 줄은 버린다. 버전 칸이 비어 있으면 `version_output`은 `None`이다.
/// 같은 에이전트가 여러 줄이면 첫 줄(PATH에서 먼저 찾은 것)을 쓴다.
pub fn parse_probe_output(stdout: &str, marker: &str) -> BTreeMap<AgentKind, Found> {
    stdout
        .lines()
        .filter_map(|line| probe_line(line.trim_end_matches('\r'), marker))
        .fold(BTreeMap::new(), |mut found, (agent, entry)| {
            found.entry(agent).or_insert(entry);
            found
        })
}

fn probe_line(line: &str, marker: &str) -> Option<(AgentKind, Found)> {
    let mut fields = line
        .strip_prefix(marker)?
        .strip_prefix('\t')?
        .splitn(3, '\t');
    let command = fields.next()?;
    let agent = AgentKind::ALL
        .into_iter()
        .find(|agent| agent.command() == command)?;
    let path = fields.next().filter(|path| path.starts_with('/'))?;
    let version_output = fields
        .next()
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(str::to_owned);
    Some((
        agent,
        Found {
            path: path.to_owned(),
            version_output,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARKER: &str = "__NW_PROBE__";

    fn utf16le(text: &str, bom: bool) -> Vec<u8> {
        let prefix: &[u8] = if bom { &[0xFF, 0xFE] } else { &[] };
        prefix
            .iter()
            .copied()
            .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
            .collect()
    }

    #[test]
    fn fr_agent_002_reads_distros_from_utf16le_with_bom() {
        let raw = utf16le("Ubuntu\r\nDebian\r\n", true);
        assert_eq!(parse_distro_list(&raw), vec!["Ubuntu", "Debian"]);
    }

    #[test]
    fn fr_agent_002_reads_distros_from_utf16le_without_bom() {
        let raw = utf16le("Ubuntu\r\nDebian\r\n", false);
        assert_eq!(parse_distro_list(&raw), vec!["Ubuntu", "Debian"]);
    }

    #[test]
    fn fr_agent_002_reads_distros_from_utf8() {
        assert_eq!(
            parse_distro_list(b"Ubuntu\nDebian\n"),
            vec!["Ubuntu", "Debian"]
        );
    }

    #[test]
    fn fr_agent_002_drops_empty_lines_and_nul() {
        assert_eq!(
            parse_distro_list(b"\nUbuntu\n\n\r\nDebian\r\n\r\n"),
            vec!["Ubuntu", "Debian"]
        );
        let raw = utf16le("\r\nUbuntu\0\r\n\0\r\nDebian\r\n\0", true);
        assert_eq!(parse_distro_list(&raw), vec!["Ubuntu", "Debian"]);
    }

    #[test]
    fn fr_agent_002_skips_tool_distros() {
        let raw = utf16le("docker-desktop\r\nUbuntu\r\ndocker-desktop-data\r\n", true);
        assert_eq!(parse_distro_list(&raw), vec!["Ubuntu"]);
    }

    #[test]
    fn fr_agent_002_no_distros_is_empty_list() {
        assert_eq!(parse_distro_list(b""), Vec::<String>::new());
        assert_eq!(parse_distro_list(&utf16le("", true)), Vec::<String>::new());
    }

    #[test]
    fn fr_agent_003_reads_claude_version() {
        assert_eq!(
            parse_version("2.1.322 (Claude Code)"),
            Some("2.1.322".to_string())
        );
    }

    #[test]
    fn fr_agent_003_reads_codex_version() {
        assert_eq!(
            parse_version("codex-cli 0.146.0"),
            Some("0.146.0".to_string())
        );
    }

    #[test]
    fn fr_agent_003_uses_first_line_only() {
        assert_eq!(
            parse_version("codex-cli 0.146.0\nbuilt with 1.2.3"),
            Some("0.146.0".to_string())
        );
        assert_eq!(parse_version("no version here\n1.2.3"), None);
    }

    #[test]
    fn fr_agent_003_empty_or_versionless_output_is_none() {
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("command not found"), None);
    }

    #[test]
    fn fr_agent_004_reads_path_between_markers() {
        let stdout = format!("{MARKER}/Users/u/.local/bin:/usr/bin{MARKER}");
        assert_eq!(
            parse_marked_path(&stdout, MARKER),
            Some("/Users/u/.local/bin:/usr/bin".to_string())
        );
    }

    #[test]
    fn fr_agent_004_ignores_text_printed_by_shell_config() {
        let stdout =
            format!("Welcome!\nnvm loaded\n{MARKER}/Users/u/.local/bin:/usr/bin{MARKER}\nbye\n");
        assert_eq!(
            parse_marked_path(&stdout, MARKER),
            Some("/Users/u/.local/bin:/usr/bin".to_string())
        );
    }

    #[test]
    fn fr_agent_004_without_two_markers_is_none() {
        assert_eq!(parse_marked_path("/usr/bin:/bin", MARKER), None);
        assert_eq!(
            parse_marked_path(&format!("{MARKER}/usr/bin:/bin"), MARKER),
            None
        );
    }

    #[test]
    fn fr_agent_003_wsl_probe_line_gives_path_and_version_output() {
        let stdout =
            format!("{MARKER}\tclaude\t/home/user/.local/bin/claude\t2.1.322 (Claude Code)\n");
        let found = parse_probe_output(&stdout, MARKER);
        assert_eq!(
            found,
            BTreeMap::from([(
                AgentKind::Claude,
                Found {
                    path: "/home/user/.local/bin/claude".to_string(),
                    version_output: Some("2.1.322 (Claude Code)".to_string()),
                },
            )])
        );
    }

    #[test]
    fn fr_agent_001_wsl_probe_reads_both_agents() {
        let stdout = format!(
            "{MARKER}\tclaude\t/home/user/.local/bin/claude\t2.1.322 (Claude Code)\n\
             {MARKER}\tcodex\t/usr/local/bin/codex\tcodex-cli 0.146.0\n"
        );
        let found = parse_probe_output(&stdout, MARKER);
        assert_eq!(
            found.get(&AgentKind::Codex),
            Some(&Found {
                path: "/usr/local/bin/codex".to_string(),
                version_output: Some("codex-cli 0.146.0".to_string()),
            })
        );
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn fr_agent_005_missing_agent_is_absent_from_probe_map() {
        let stdout = format!("{MARKER}\tclaude\t/home/user/.local/bin/claude\t2.1.322\n");
        let found = parse_probe_output(&stdout, MARKER);
        assert!(!found.contains_key(&AgentKind::Codex));
        assert!(parse_probe_output("", MARKER).is_empty());
    }

    #[test]
    fn fr_agent_003_empty_version_field_means_version_unread() {
        let stdout = format!("{MARKER}\tcodex\t/usr/local/bin/codex\t\n");
        assert_eq!(
            parse_probe_output(&stdout, MARKER).get(&AgentKind::Codex),
            Some(&Found {
                path: "/usr/local/bin/codex".to_string(),
                version_output: None,
            })
        );
    }

    #[test]
    fn fr_agent_003_ignores_lines_without_marker() {
        let stdout = format!(
            "claude\t/home/user/fake/claude\t9.9.9\n\
             motd banner\n\
             {MARKER}\tcodex\t/usr/local/bin/codex\tcodex-cli 0.146.0\n"
        );
        let found = parse_probe_output(&stdout, MARKER);
        assert!(!found.contains_key(&AgentKind::Claude));
        assert!(found.contains_key(&AgentKind::Codex));
    }

    #[test]
    fn fr_agent_003_ignores_non_absolute_paths_and_unknown_agents() {
        let stdout = format!(
            "{MARKER}\tclaude\tclaude\t2.1.322\n\
             {MARKER}\tgemini\t/usr/local/bin/gemini\t1.0.0\n"
        );
        assert!(parse_probe_output(&stdout, MARKER).is_empty());
    }

    #[test]
    fn fr_agent_003_strips_one_leading_v() {
        assert_eq!(parse_version("v2.1.322"), Some("2.1.322".to_string()));
        assert_eq!(
            parse_version("claude v2.1.322"),
            Some("2.1.322".to_string())
        );
    }

    #[test]
    fn fr_agent_003_keeps_whole_token_with_prerelease_suffix() {
        assert_eq!(
            parse_version("2.1.322-beta.1"),
            Some("2.1.322-beta.1".to_string())
        );
    }

    #[test]
    fn fr_agent_003_two_part_version_is_none() {
        assert_eq!(parse_version("2.1"), None);
    }

    #[test]
    fn fr_agent_003_uses_first_token_that_starts_with_three_numbers() {
        assert_eq!(
            parse_version("tool 2.1 build 3.4.5 (x)"),
            Some("3.4.5".to_string())
        );
    }

    #[test]
    fn fr_agent_004_more_than_two_markers_uses_first_pair() {
        let stdout = format!("{MARKER}/first:/usr/bin{MARKER}/second{MARKER}");
        assert_eq!(
            parse_marked_path(&stdout, MARKER),
            Some("/first:/usr/bin".to_string())
        );
    }

    #[test]
    fn fr_agent_003_duplicate_agent_lines_use_first() {
        let stdout = format!(
            "{MARKER}\tcodex\t/home/user/.local/bin/codex\tcodex-cli 0.146.0\n\
             {MARKER}\tcodex\t/usr/local/bin/codex\tcodex-cli 0.100.0\n"
        );
        assert_eq!(
            parse_probe_output(&stdout, MARKER).get(&AgentKind::Codex),
            Some(&Found {
                path: "/home/user/.local/bin/codex".to_string(),
                version_output: Some("codex-cli 0.146.0".to_string()),
            })
        );
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;

        const TOOL_DISTROS: [&str; 2] = ["docker-desktop", "docker-desktop-data"];

        fn distro_name() -> impl Strategy<Value = String> {
            prop_oneof![
                "[A-Za-z][A-Za-z0-9.-]{0,15}",
                Just("docker-desktop".to_string()),
                Just("docker-desktop-data".to_string()),
            ]
        }

        fn encode(names: &[String], crlf: bool, utf16: bool, bom: bool) -> Vec<u8> {
            let newline = if crlf { "\r\n" } else { "\n" };
            let text: String = names.iter().map(|n| format!("{n}{newline}")).collect();
            if utf16 {
                utf16le(&text, bom)
            } else {
                text.into_bytes()
            }
        }

        proptest! {
            #[test]
            fn fr_agent_002_distro_list_keeps_order_and_drops_tool_distros(
                names in prop::collection::vec(distro_name(), 0..8),
                crlf in any::<bool>(),
                utf16 in any::<bool>(),
                bom in any::<bool>(),
            ) {
                let expected: Vec<String> = names
                    .iter()
                    .filter(|n| !TOOL_DISTROS.contains(&n.as_str()))
                    .cloned()
                    .collect();
                prop_assert_eq!(parse_distro_list(&encode(&names, crlf, utf16, bom)), expected);
            }

            #[test]
            fn fr_agent_003_version_after_name_and_optional_v(
                name in "[a-z][a-z-]{0,10}",
                v in any::<bool>(),
                major in 0u32..1000,
                minor in 0u32..1000,
                patch in 0u32..1000,
            ) {
                let prefix = if v { "v" } else { "" };
                let output = format!("{name} {prefix}{major}.{minor}.{patch}");
                prop_assert_eq!(
                    parse_version(&output),
                    Some(format!("{major}.{minor}.{patch}"))
                );
            }

            #[test]
            fn fr_agent_004_marked_path_ignores_surrounding_text(
                before in "[a-zA-Z0-9 :/.\n]{0,40}",
                path in "[a-z0-9:/.-]{0,60}",
                after in "[a-zA-Z0-9 :/.\n]{0,40}",
            ) {
                let stdout = format!("{before}{MARKER}{path}{MARKER}{after}");
                prop_assert_eq!(parse_marked_path(&stdout, MARKER), Some(path));
            }
        }
    }
}
