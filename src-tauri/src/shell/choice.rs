use super::model::ShellKind;
use serde::{Deserialize, Serialize};

/// `shell.json`의 모양.
#[derive(Serialize, Deserialize)]
struct SavedShell {
    shell: ShellKind,
}

const BASH_UNDER_GIT: &str = "\\bin\\bash.exe";

/// `shell.json` 내용에서 저장한 셸을 읽는다. 모르는 값이거나 JSON이 아니면 None.
pub fn parse_shell_choice(contents: &str) -> Option<ShellKind> {
    serde_json::from_str::<SavedShell>(contents)
        .ok()
        .map(|saved| saved.shell)
}

/// `shell.json`에 쓸 내용. `parse_shell_choice`로 다시 읽을 수 있다.
pub fn shell_choice_json(shell: ShellKind) -> String {
    serde_json::json!({ "shell": shell }).to_string()
}

/// 실제로 쓸 셸. 저장한 값이 없으면 PowerShell, Git Bash인데 없으면 PowerShell(FR-START-012).
pub fn effective_shell(saved: Option<ShellKind>, git_bash_available: bool) -> ShellKind {
    match saved {
        Some(ShellKind::GitBash) if !git_bash_available => ShellKind::PowerShell,
        Some(shell) => shell,
        None => ShellKind::PowerShell,
    }
}

/// 선택지. [PowerShell, Cmd] 뒤에 Git Bash가 있으면 GitBash(FR-START-011).
pub fn shell_options(git_bash_available: bool) -> Vec<ShellKind> {
    let git_bash = git_bash_available.then_some(ShellKind::GitBash);
    [ShellKind::PowerShell, ShellKind::Cmd]
        .into_iter()
        .chain(git_bash)
        .collect()
}

/// Git for Windows bash.exe 후보. `ProgramFiles`, `ProgramW6432`, `ProgramFiles(x86)` 아래 `Git\bin\bash.exe`,
/// `LOCALAPPDATA\Programs\Git\bin\bash.exe`, `PATH`의 `...\Git\cmd`에 대응하는 `...\Git\bin\bash.exe` 순서. 중복 없음.
pub fn git_bash_candidates(
    program_files: &[&str],
    local_app_data: Option<&str>,
    path_env: &str,
) -> Vec<String> {
    let installed = program_files
        .iter()
        .map(|dir| format!("{}\\Git{BASH_UNDER_GIT}", trim_separator(dir)));
    let per_user =
        local_app_data.map(|dir| format!("{}\\Programs\\Git{BASH_UNDER_GIT}", trim_separator(dir)));
    let on_path = path_env
        .split(';')
        .filter_map(git_dir_of_cmd)
        .map(|git| format!("{git}{BASH_UNDER_GIT}"));
    installed.chain(per_user).chain(on_path).fold(
        Vec::new(),
        |mut unique: Vec<String>, candidate| {
            // Windows 경로는 대소문자를 구분하지 않으므로 먼저 나온 표기만 남긴다
            if !unique
                .iter()
                .any(|seen| seen.eq_ignore_ascii_case(&candidate))
            {
                unique.push(candidate);
            }
            unique
        },
    )
}

fn trim_separator(dir: &str) -> &str {
    dir.trim_end_matches('\\')
}

/// `PATH` 항목이 `...\Git\cmd`면 `...\Git`. 대소문자와 끝 구분자는 가리지 않는다.
fn git_dir_of_cmd(entry: &str) -> Option<&str> {
    const SUFFIX: &str = "\\git\\cmd";
    let entry = trim_separator(entry.trim());
    let split = entry.len().checked_sub(SUFFIX.len())?;
    let (_, tail) = entry.split_at_checked(split)?;
    tail.eq_ignore_ascii_case(SUFFIX)
        .then(|| &entry[..split + "\\git".len()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const ALL: [ShellKind; 3] = [ShellKind::PowerShell, ShellKind::Cmd, ShellKind::GitBash];
    const BASH_SUFFIX: &str = "\\Git\\bin\\bash.exe";

    // parse_shell_choice, shell_choice_json

    #[test]
    fn fr_start_012_saved_choice_reads_back_as_the_same_shell() {
        for shell in ALL {
            assert_eq!(
                parse_shell_choice(&shell_choice_json(shell)),
                Some(shell),
                "{shell:?}"
            );
        }
    }

    #[test]
    fn fr_start_012_saved_choices_differ_per_shell() {
        assert_ne!(
            shell_choice_json(ShellKind::PowerShell),
            shell_choice_json(ShellKind::Cmd)
        );
        assert_ne!(
            shell_choice_json(ShellKind::Cmd),
            shell_choice_json(ShellKind::GitBash)
        );
        assert_ne!(
            shell_choice_json(ShellKind::PowerShell),
            shell_choice_json(ShellKind::GitBash)
        );
    }

    #[test]
    fn fr_start_012_choice_file_that_is_not_json_is_none() {
        assert_eq!(parse_shell_choice(""), None);
        assert_eq!(parse_shell_choice("not json"), None);
    }

    // effective_shell

    #[test]
    fn fr_start_012_no_saved_choice_is_powershell() {
        assert_eq!(effective_shell(None, true), ShellKind::PowerShell);
        assert_eq!(effective_shell(None, false), ShellKind::PowerShell);
    }

    #[test]
    fn fr_start_012_scenario_1_saved_git_bash_still_installed_is_git_bash() {
        assert_eq!(
            effective_shell(Some(ShellKind::GitBash), true),
            ShellKind::GitBash
        );
    }

    #[test]
    fn fr_start_012_scenario_2_saved_git_bash_removed_is_powershell() {
        assert_eq!(
            effective_shell(Some(ShellKind::GitBash), false),
            ShellKind::PowerShell
        );
    }

    #[test]
    fn fr_start_012_saved_powershell_or_cmd_is_kept() {
        for available in [true, false] {
            assert_eq!(
                effective_shell(Some(ShellKind::PowerShell), available),
                ShellKind::PowerShell
            );
            assert_eq!(
                effective_shell(Some(ShellKind::Cmd), available),
                ShellKind::Cmd
            );
        }
    }

    #[test]
    fn fr_start_012_effective_shell_is_always_an_option() {
        for available in [true, false] {
            for saved in [
                None,
                Some(ShellKind::PowerShell),
                Some(ShellKind::Cmd),
                Some(ShellKind::GitBash),
            ] {
                assert!(
                    shell_options(available).contains(&effective_shell(saved, available)),
                    "{saved:?}, {available}"
                );
            }
        }
    }

    // shell_options

    #[test]
    fn fr_start_011_scenario_1_with_git_bash_offers_three_shells() {
        assert_eq!(
            shell_options(true),
            vec![ShellKind::PowerShell, ShellKind::Cmd, ShellKind::GitBash]
        );
    }

    #[test]
    fn fr_start_011_scenario_2_without_git_bash_offers_powershell_and_cmd() {
        assert_eq!(
            shell_options(false),
            vec![ShellKind::PowerShell, ShellKind::Cmd]
        );
    }

    // git_bash_candidates

    #[test]
    fn fr_start_011_git_bash_candidates_in_documented_order() {
        let got = git_bash_candidates(
            &["C:\\Program Files", "D:\\Apps", "C:\\Program Files (x86)"],
            Some("C:\\Users\\a\\AppData\\Local"),
            "C:\\Windows\\System32;E:\\Tools\\Git\\cmd;C:\\Windows",
        );

        assert_eq!(
            got,
            vec![
                "C:\\Program Files\\Git\\bin\\bash.exe".to_string(),
                "D:\\Apps\\Git\\bin\\bash.exe".to_string(),
                "C:\\Program Files (x86)\\Git\\bin\\bash.exe".to_string(),
                "C:\\Users\\a\\AppData\\Local\\Programs\\Git\\bin\\bash.exe".to_string(),
                "E:\\Tools\\Git\\bin\\bash.exe".to_string(),
            ]
        );
    }

    #[test]
    fn fr_start_011_git_bash_candidates_drop_duplicates_keeping_first() {
        // ProgramFiles와 ProgramW6432는 보통 같은 폴더이고, PATH의 Git\cmd도 같은 설치를 가리킨다
        let got = git_bash_candidates(
            &[
                "C:\\Program Files",
                "C:\\Program Files",
                "C:\\Program Files (x86)",
            ],
            None,
            "C:\\Program Files\\Git\\cmd;C:\\Windows",
        );

        assert_eq!(
            got,
            vec![
                "C:\\Program Files\\Git\\bin\\bash.exe".to_string(),
                "C:\\Program Files (x86)\\Git\\bin\\bash.exe".to_string(),
            ]
        );
    }

    #[test]
    fn fr_start_011_git_bash_candidates_without_localappdata() {
        let got = git_bash_candidates(&["C:\\Program Files"], None, "");

        assert_eq!(
            got,
            vec!["C:\\Program Files\\Git\\bin\\bash.exe".to_string()]
        );
    }

    #[test]
    fn fr_start_011_git_bash_candidates_ignore_path_entries_without_git_cmd() {
        let got = git_bash_candidates(&[], None, "C:\\Windows;C:\\Windows\\System32;D:\\node");

        assert_eq!(got, Vec::<String>::new());
    }

    fn program_dir() -> impl Strategy<Value = String> {
        "[C-E]:\\\\[A-Za-z ]{1,8}"
    }

    fn path_entry() -> impl Strategy<Value = String> {
        prop_oneof!["[C-E]:\\\\[a-z]{1,5}\\\\Git\\\\cmd", "[C-E]:\\\\[a-z]{1,5}",]
    }

    proptest! {
        #[test]
        fn fr_start_011_git_bash_candidates_are_unique_bash_paths(
            program_files in prop::collection::vec(program_dir(), 0..5),
            local in prop::option::of("[C-E]:\\\\Users\\\\[a-z]{1,5}\\\\AppData\\\\Local"),
            path in prop::collection::vec(path_entry(), 0..5),
        ) {
            let dirs: Vec<&str> = program_files.iter().map(String::as_str).collect();
            let got = git_bash_candidates(&dirs, local.as_deref(), &path.join(";"));

            // Windows 경로는 대소문자를 구분하지 않으므로 중복도 대소문자를 무시하고 본다
            let lowered: Vec<String> = got.iter().map(|c| c.to_ascii_lowercase()).collect();
            let unique = lowered.iter().collect::<std::collections::BTreeSet<_>>();
            prop_assert_eq!(unique.len(), got.len());
            for candidate in &lowered {
                prop_assert!(candidate.ends_with(&BASH_SUFFIX.to_ascii_lowercase()), "{}", candidate);
            }
            for dir in &program_files {
                let expected = format!("{dir}{BASH_SUFFIX}").to_ascii_lowercase();
                prop_assert!(lowered.contains(&expected), "{}", expected);
            }
            if let Some(local) = &local {
                let expected = format!("{local}\\Programs{BASH_SUFFIX}").to_ascii_lowercase();
                prop_assert!(lowered.contains(&expected), "{}", expected);
            }
        }

        #[test]
        fn fr_start_011_git_bash_candidates_start_with_program_files_in_order(
            program_files in prop::collection::vec(program_dir(), 0..5),
        ) {
            let dirs: Vec<&str> = program_files.iter().map(String::as_str).collect();
            let got = git_bash_candidates(&dirs, None, "");

            let mut expected: Vec<String> = Vec::new();
            for dir in &program_files {
                let candidate = format!("{dir}{BASH_SUFFIX}");
                if !expected.iter().any(|e| e.eq_ignore_ascii_case(&candidate)) {
                    expected.push(candidate);
                }
            }
            prop_assert_eq!(got, expected);
        }
    }

    #[test]
    fn fr_start_011_git_bash_candidates_path_entry_in_other_case() {
        let got = git_bash_candidates(&[], None, "C:\\Windows;C:\\Program Files\\git\\CMD");

        assert_eq!(got.len(), 1, "{got:?}");
        assert!(
            got[0].eq_ignore_ascii_case("C:\\Program Files\\git\\bin\\bash.exe"),
            "{got:?}"
        );
    }

    #[test]
    fn fr_start_011_git_bash_candidates_path_entry_with_trailing_separator() {
        let got = git_bash_candidates(&[], None, "C:\\Program Files\\Git\\cmd\\;C:\\Windows");

        assert_eq!(
            got,
            vec!["C:\\Program Files\\Git\\bin\\bash.exe".to_string()]
        );
    }

    #[test]
    fn fr_start_011_git_bash_candidates_program_files_with_trailing_separator() {
        let got = git_bash_candidates(&["C:\\Program Files\\"], None, "");

        assert_eq!(
            got,
            vec!["C:\\Program Files\\Git\\bin\\bash.exe".to_string()]
        );
    }

    #[test]
    fn fr_start_011_git_bash_candidates_drop_duplicates_ignoring_case_keeping_first() {
        let got = git_bash_candidates(
            &["C:\\PROGRAM FILES", "C:\\Program Files"],
            None,
            "c:\\program files\\git\\cmd",
        );

        assert_eq!(
            got,
            vec!["C:\\PROGRAM FILES\\Git\\bin\\bash.exe".to_string()]
        );
    }

    #[test]
    fn fr_start_011_git_bash_candidates_trailing_separator_duplicates_are_dropped() {
        let got = git_bash_candidates(
            &["C:\\Program Files\\", "C:\\Program Files"],
            None,
            "C:\\Program Files\\Git\\cmd\\",
        );

        assert_eq!(
            got,
            vec!["C:\\Program Files\\Git\\bin\\bash.exe".to_string()]
        );
    }
}
