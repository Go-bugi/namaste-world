use super::model::{
    AgentEntry, AgentKind, AgentStatus, Detection, EnvProbe, Environment, FailureReason, HostOs,
    Target, Trigger,
};
use super::parse::parse_version;
use std::path::{Path, PathBuf};

/// 이 환경을 프로브할지 정한다. 실행 중이면 항상, 멈췄으면 사용자가 요청한 감지일 때만 프로브한다.
pub fn should_probe(target: &Target, trigger: Trigger) -> bool {
    match trigger {
        Trigger::Startup => target.running,
        Trigger::UserRequested => true,
    }
}

/// 프로브하지 않은 환경의 항목. Claude, Codex 순서로 두 항목 모두 `Stopped`다.
pub fn stopped_entries(environment: &Environment) -> Vec<AgentEntry> {
    entries_with(environment, |_| AgentStatus::Stopped)
}

/// 한 환경의 프로브 결과를 항목으로 바꾼다. 항상 Claude, Codex 순서로 두 항목을 낸다.
/// `Err(reason)` → 두 항목 모두 `Failed(reason)` / 맵에 없음 → `NotInstalled` /
/// 버전을 뽑지 못함 → `Failed(VersionUnreadable)` / 그 밖 → `Installed`
pub fn entries_for(environment: &Environment, probe: &EnvProbe) -> Vec<AgentEntry> {
    entries_with(environment, |agent| match probe {
        Err(reason) => AgentStatus::Failed {
            reason: reason.clone(),
        },
        Ok(found) => match found.get(&agent) {
            None => AgentStatus::NotInstalled,
            Some(found) => match found.version_output.as_deref().and_then(parse_version) {
                Some(version) => AgentStatus::Installed {
                    path: found.path.clone(),
                    version,
                },
                None => AgentStatus::Failed {
                    reason: FailureReason::VersionUnreadable,
                },
            },
        },
    })
}

fn entries_with(
    environment: &Environment,
    status_of: impl Fn(AgentKind) -> AgentStatus,
) -> Vec<AgentEntry> {
    AgentKind::ALL
        .into_iter()
        .map(|agent| AgentEntry {
            agent,
            environment: environment.clone(),
            status: status_of(agent),
        })
        .collect()
}

/// 실행 파일을 찾을 디렉터리. `PATH` 항목을 먼저, 설치 방법별 기본 폴더를 뒤에 두고 중복은 뺀다.
/// `path_env`는 OS의 구분자(macOS `:`, Windows `;`)로 나누고 빈 항목은 버린다.
/// 기본 폴더는 OS와 무관하게 `Path::join`으로 만든다.
///
/// - macOS 기본 폴더: `~/.local/bin`, `~/.claude/local`, `/opt/homebrew/bin`, `/usr/local/bin`,
///   `~/.npm-global/bin`, `~/.bun/bin`, `~/.volta/bin`
/// - Windows 기본 폴더: `%USERPROFILE%\.local\bin`, `%APPDATA%\npm`, `%LOCALAPPDATA%\Microsoft\WinGet\Links`.
///   `app_data`, `local_app_data`가 `None`이면 그 폴더는 뺀다.
pub fn search_dirs(
    os: HostOs,
    path_env: &str,
    home: &Path,
    app_data: Option<&Path>,
    local_app_data: Option<&Path>,
) -> Vec<PathBuf> {
    let separator = match os {
        HostOs::Macos => ':',
        HostOs::Windows => ';',
    };
    let defaults = match os {
        HostOs::Macos => vec![
            home.join(".local").join("bin"),
            home.join(".claude").join("local"),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            home.join(".npm-global").join("bin"),
            home.join(".bun").join("bin"),
            home.join(".volta").join("bin"),
        ],
        HostOs::Windows => [
            Some(home.join(".local").join("bin")),
            app_data.map(|dir| dir.join("npm")),
            local_app_data.map(|dir| dir.join("Microsoft").join("WinGet").join("Links")),
        ]
        .into_iter()
        .flatten()
        .collect(),
    };
    path_env
        .split(separator)
        .filter(|entry| !entry.is_empty())
        .map(PathBuf::from)
        .chain(defaults)
        .fold(Vec::new(), |mut dirs, dir| {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
            dirs
        })
}

/// 디렉터리에서 찾아볼 파일 이름. macOS는 `[name]`, Windows는 `[name.exe, name.cmd]` 순서다.
pub fn executable_names(os: HostOs, agent: AgentKind) -> Vec<String> {
    let name = agent.command();
    match os {
        HostOs::Macos => vec![name.to_owned()],
        HostOs::Windows => vec![format!("{name}.exe"), format!("{name}.cmd")],
    }
}

/// 감지 결과에서 `agent`와 `environment`가 같고 설치됨인 항목의 실행 파일 경로. 없으면 None.
pub fn installed_path(
    detection: &Detection,
    agent: AgentKind,
    environment: &Environment,
) -> Option<String> {
    detection
        .entries
        .iter()
        .find_map(|entry| match &entry.status {
            AgentStatus::Installed { path, .. }
                if entry.agent == agent && &entry.environment == environment =>
            {
                Some(path.clone())
            }
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::domain::model::{AgentStatus, FailureReason, Found};
    use std::collections::BTreeMap;

    fn ubuntu() -> Environment {
        Environment::Wsl {
            distro: "Ubuntu".to_string(),
        }
    }

    fn entry(agent: AgentKind, environment: &Environment, status: AgentStatus) -> AgentEntry {
        AgentEntry {
            agent,
            environment: environment.clone(),
            status,
        }
    }

    fn found(path: &str, version_output: Option<&str>) -> Found {
        Found {
            path: path.to_string(),
            version_output: version_output.map(str::to_string),
        }
    }

    #[test]
    fn fr_agent_002_running_environment_is_always_probed() {
        for environment in [Environment::Macos, Environment::Windows, ubuntu()] {
            let target = Target {
                environment,
                running: true,
            };
            assert!(should_probe(&target, Trigger::Startup));
            assert!(should_probe(&target, Trigger::UserRequested));
        }
    }

    #[test]
    fn fr_agent_002_stopped_distro_is_not_started_on_startup() {
        let target = Target {
            environment: ubuntu(),
            running: false,
        };
        assert!(!should_probe(&target, Trigger::Startup));
    }

    #[test]
    fn fr_agent_002_stopped_distro_is_probed_on_redetect() {
        let target = Target {
            environment: ubuntu(),
            running: false,
        };
        assert!(should_probe(&target, Trigger::UserRequested));
    }

    #[test]
    fn fr_agent_002_stopped_entries_mark_both_agents_stopped() {
        let env = ubuntu();
        assert_eq!(
            stopped_entries(&env),
            vec![
                entry(AgentKind::Claude, &env, AgentStatus::Stopped),
                entry(AgentKind::Codex, &env, AgentStatus::Stopped),
            ]
        );
    }

    #[test]
    fn fr_agent_001_both_found_are_installed() {
        let env = Environment::Macos;
        let probe: EnvProbe = Ok(BTreeMap::from([
            (
                AgentKind::Claude,
                found("/Users/u/.local/bin/claude", Some("2.1.322 (Claude Code)")),
            ),
            (
                AgentKind::Codex,
                found("/opt/homebrew/bin/codex", Some("codex-cli 0.146.0")),
            ),
        ]));
        assert_eq!(
            entries_for(&env, &probe),
            vec![
                entry(
                    AgentKind::Claude,
                    &env,
                    AgentStatus::Installed {
                        path: "/Users/u/.local/bin/claude".to_string(),
                        version: "2.1.322".to_string(),
                    }
                ),
                entry(
                    AgentKind::Codex,
                    &env,
                    AgentStatus::Installed {
                        path: "/opt/homebrew/bin/codex".to_string(),
                        version: "0.146.0".to_string(),
                    }
                ),
            ]
        );
    }

    #[test]
    fn fr_agent_001_only_claude_found_codex_not_installed() {
        let env = Environment::Macos;
        let probe: EnvProbe = Ok(BTreeMap::from([(
            AgentKind::Claude,
            found("/Users/u/.local/bin/claude", Some("2.1.322 (Claude Code)")),
        )]));
        let entries = entries_for(&env, &probe);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].agent, AgentKind::Claude);
        assert!(matches!(entries[0].status, AgentStatus::Installed { .. }));
        assert_eq!(
            entries[1],
            entry(AgentKind::Codex, &env, AgentStatus::NotInstalled)
        );
    }

    #[test]
    fn fr_agent_003_wsl_entry_has_distro_path_and_version() {
        let env = ubuntu();
        let probe: EnvProbe = Ok(BTreeMap::from([(
            AgentKind::Claude,
            found(
                "/home/user/.local/bin/claude",
                Some("2.1.322 (Claude Code)"),
            ),
        )]));
        assert_eq!(
            entries_for(&env, &probe)[0],
            entry(
                AgentKind::Claude,
                &env,
                AgentStatus::Installed {
                    path: "/home/user/.local/bin/claude".to_string(),
                    version: "2.1.322".to_string(),
                }
            )
        );
    }

    #[test]
    fn fr_agent_003_failed_version_command_is_version_unreadable() {
        let env = Environment::Windows;
        let probe: EnvProbe = Ok(BTreeMap::from([(
            AgentKind::Codex,
            found("C:\\bin\\codex.exe", None),
        )]));
        assert_eq!(
            entries_for(&env, &probe)[1],
            entry(
                AgentKind::Codex,
                &env,
                AgentStatus::Failed {
                    reason: FailureReason::VersionUnreadable
                }
            )
        );
    }

    #[test]
    fn fr_agent_003_unparseable_version_is_version_unreadable() {
        let env = Environment::Windows;
        let probe: EnvProbe = Ok(BTreeMap::from([(
            AgentKind::Claude,
            found("C:\\bin\\claude.exe", Some("error: something broke")),
        )]));
        assert_eq!(
            entries_for(&env, &probe)[0].status,
            AgentStatus::Failed {
                reason: FailureReason::VersionUnreadable
            }
        );
    }

    #[test]
    fn fr_agent_005_environment_failure_marks_both_failed() {
        let env = ubuntu();
        let probe: EnvProbe = Err(FailureReason::Timeout);
        assert_eq!(
            entries_for(&env, &probe),
            vec![
                entry(
                    AgentKind::Claude,
                    &env,
                    AgentStatus::Failed {
                        reason: FailureReason::Timeout
                    }
                ),
                entry(
                    AgentKind::Codex,
                    &env,
                    AgentStatus::Failed {
                        reason: FailureReason::Timeout
                    }
                ),
            ]
        );
    }

    #[test]
    fn fr_agent_005_environment_error_keeps_message() {
        let env = ubuntu();
        let reason = FailureReason::EnvironmentError {
            message: "wsl.exe exited with 1".to_string(),
        };
        let entries = entries_for(&env, &Err(reason.clone()));
        assert!(entries.iter().all(|e| e.status
            == AgentStatus::Failed {
                reason: reason.clone()
            }));
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn fr_agent_005_missing_executable_is_not_installed_not_failed() {
        let env = Environment::Windows;
        let probe: EnvProbe = Ok(BTreeMap::new());
        assert_eq!(
            entries_for(&env, &probe),
            vec![
                entry(AgentKind::Claude, &env, AgentStatus::NotInstalled),
                entry(AgentKind::Codex, &env, AgentStatus::NotInstalled),
            ]
        );
    }

    #[test]
    fn fr_agent_004_macos_path_entries_come_before_default_dirs() {
        let home = Path::new("/Users/u");
        let dirs = search_dirs(HostOs::Macos, "/usr/bin:/bin", home, None, None);
        assert_eq!(
            dirs,
            vec![
                PathBuf::from("/usr/bin"),
                PathBuf::from("/bin"),
                home.join(".local").join("bin"),
                home.join(".claude").join("local"),
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/bin"),
                home.join(".npm-global").join("bin"),
                home.join(".bun").join("bin"),
                home.join(".volta").join("bin"),
            ]
        );
    }

    #[test]
    fn fr_agent_004_shell_config_dir_in_path_is_searched() {
        let home = Path::new("/Users/u");
        let dirs = search_dirs(
            HostOs::Macos,
            "/Users/u/custom/bin:/usr/bin",
            home,
            None,
            None,
        );
        assert_eq!(dirs[0], PathBuf::from("/Users/u/custom/bin"));
    }

    #[test]
    fn fr_agent_001_macos_dirs_have_no_duplicates_and_no_empty_entries() {
        let home = Path::new("/Users/u");
        let dirs = search_dirs(
            HostOs::Macos,
            "::/opt/homebrew/bin:/usr/bin:/usr/bin:/Users/u/.local/bin:",
            home,
            None,
            None,
        );
        assert_eq!(
            dirs,
            vec![
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/bin"),
                home.join(".local").join("bin"),
                home.join(".claude").join("local"),
                PathBuf::from("/usr/local/bin"),
                home.join(".npm-global").join("bin"),
                home.join(".bun").join("bin"),
                home.join(".volta").join("bin"),
            ]
        );
    }

    #[test]
    fn fr_agent_001_windows_dirs_include_npm_and_winget() {
        let home = Path::new("C:\\Users\\u");
        let app_data = Path::new("C:\\Users\\u\\AppData\\Roaming");
        let local_app_data = Path::new("C:\\Users\\u\\AppData\\Local");
        let dirs = search_dirs(
            HostOs::Windows,
            "C:\\Windows;;C:\\tools",
            home,
            Some(app_data),
            Some(local_app_data),
        );
        assert_eq!(
            dirs,
            vec![
                PathBuf::from("C:\\Windows"),
                PathBuf::from("C:\\tools"),
                home.join(".local").join("bin"),
                app_data.join("npm"),
                local_app_data
                    .join("Microsoft")
                    .join("WinGet")
                    .join("Links"),
            ]
        );
    }

    #[test]
    fn fr_agent_001_windows_dirs_skip_missing_app_data() {
        let home = Path::new("C:\\Users\\u");
        let dirs = search_dirs(HostOs::Windows, "", home, None, None);
        assert_eq!(dirs, vec![home.join(".local").join("bin")]);
    }

    #[test]
    fn fr_agent_001_executable_names_per_os() {
        assert_eq!(
            executable_names(HostOs::Macos, AgentKind::Claude),
            vec!["claude"]
        );
        assert_eq!(
            executable_names(HostOs::Macos, AgentKind::Codex),
            vec!["codex"]
        );
        assert_eq!(
            executable_names(HostOs::Windows, AgentKind::Claude),
            vec!["claude.exe", "claude.cmd"]
        );
        assert_eq!(
            executable_names(HostOs::Windows, AgentKind::Codex),
            vec!["codex.exe", "codex.cmd"]
        );
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;

        fn macos_defaults(home: &Path) -> Vec<PathBuf> {
            vec![
                home.join(".local").join("bin"),
                home.join(".claude").join("local"),
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/bin"),
                home.join(".npm-global").join("bin"),
                home.join(".bun").join("bin"),
                home.join(".volta").join("bin"),
            ]
        }

        /// 처음 나온 것만 남긴다.
        fn first_occurrences(dirs: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
            dirs.into_iter().fold(Vec::new(), |mut kept, dir| {
                if !kept.contains(&dir) {
                    kept.push(dir);
                }
                kept
            })
        }

        fn path_entry() -> impl Strategy<Value = String> {
            prop_oneof![
                "/[a-z]{1,6}(/[a-z]{1,6}){0,2}",
                Just(String::new()),
                Just("/usr/local/bin".to_string()),
                Just("/opt/homebrew/bin".to_string()),
                Just("/Users/u/.local/bin".to_string()),
            ]
        }

        proptest! {
            #[test]
            fn fr_agent_001_path_first_then_defaults_without_duplicates(
                entries in prop::collection::vec(path_entry(), 0..10),
            ) {
                let home = Path::new("/Users/u");
                let path_env = entries.join(":");
                let expected = first_occurrences(
                    entries
                        .iter()
                        .filter(|e| !e.is_empty())
                        .map(PathBuf::from)
                        .chain(macos_defaults(home)),
                );
                prop_assert_eq!(
                    search_dirs(HostOs::Macos, &path_env, home, None, None),
                    expected
                );
            }
        }
    }
}

#[cfg(test)]
mod installed_path_tests {
    use super::*;
    use crate::agent::domain::model::FailureReason;
    use proptest::prelude::*;

    fn ubuntu() -> Environment {
        Environment::Wsl {
            distro: "Ubuntu".to_string(),
        }
    }

    fn installed(path: &str) -> AgentStatus {
        AgentStatus::Installed {
            path: path.to_string(),
            version: "1.0.0".to_string(),
        }
    }

    fn entry(agent: AgentKind, environment: Environment, status: AgentStatus) -> AgentEntry {
        AgentEntry {
            agent,
            environment,
            status,
        }
    }

    /// Windows의 Codex는 설치됨, WSL `Ubuntu`의 Codex는 다른 경로로 설치됨, macOS의 Claude Code는 설치되지 않음
    fn sample() -> Detection {
        Detection {
            entries: vec![
                entry(
                    AgentKind::Claude,
                    Environment::Macos,
                    AgentStatus::NotInstalled,
                ),
                entry(
                    AgentKind::Codex,
                    Environment::Windows,
                    installed("C:\\bin\\codex.exe"),
                ),
                entry(AgentKind::Codex, ubuntu(), installed("/usr/bin/codex")),
            ],
            shell_env_failure: None,
        }
    }

    #[test]
    fn fr_agent_009_gives_path_of_installed_entry() {
        assert_eq!(
            installed_path(&sample(), AgentKind::Codex, &Environment::Windows),
            Some("C:\\bin\\codex.exe".to_string())
        );
    }

    #[test]
    fn fr_agent_009_picks_path_of_the_requested_environment() {
        // 같은 Codex라도 WSL `Ubuntu` 항목은 그 배포판 안의 경로다
        assert_eq!(
            installed_path(&sample(), AgentKind::Codex, &ubuntu()),
            Some("/usr/bin/codex".to_string())
        );
    }

    #[test]
    fn fr_agent_009_other_distro_has_no_path() {
        let debian = Environment::Wsl {
            distro: "Debian".to_string(),
        };
        assert_eq!(installed_path(&sample(), AgentKind::Codex, &debian), None);
    }

    #[test]
    fn fr_agent_009_not_installed_entry_has_no_path() {
        assert_eq!(
            installed_path(&sample(), AgentKind::Claude, &Environment::Macos),
            None
        );
    }

    #[test]
    fn fr_agent_009_failed_and_stopped_entries_have_no_path() {
        let detection = Detection {
            entries: vec![
                entry(
                    AgentKind::Claude,
                    ubuntu(),
                    AgentStatus::Failed {
                        reason: FailureReason::Timeout,
                    },
                ),
                entry(AgentKind::Codex, ubuntu(), AgentStatus::Stopped),
            ],
            shell_env_failure: None,
        };
        assert_eq!(
            installed_path(&detection, AgentKind::Claude, &ubuntu()),
            None
        );
        assert_eq!(
            installed_path(&detection, AgentKind::Codex, &ubuntu()),
            None
        );
    }

    #[test]
    fn fr_agent_009_missing_entry_has_no_path() {
        let empty = Detection {
            entries: vec![],
            shell_env_failure: None,
        };
        assert_eq!(
            installed_path(&empty, AgentKind::Codex, &Environment::Macos),
            None
        );
        assert_eq!(
            installed_path(&sample(), AgentKind::Claude, &Environment::Windows),
            None
        );
    }

    fn environment_strategy() -> impl Strategy<Value = Environment> {
        prop_oneof![
            Just(Environment::Macos),
            Just(Environment::Windows),
            Just(ubuntu()),
            Just(Environment::Wsl {
                distro: "Debian".to_string()
            }),
        ]
    }

    fn agent_strategy() -> impl Strategy<Value = AgentKind> {
        prop_oneof![Just(AgentKind::Claude), Just(AgentKind::Codex)]
    }

    fn status_strategy() -> impl Strategy<Value = AgentStatus> {
        prop_oneof![
            "[a-z/]{1,12}".prop_map(|path| installed(&path)),
            Just(AgentStatus::NotInstalled),
            Just(AgentStatus::Failed {
                reason: FailureReason::VersionUnreadable
            }),
            Just(AgentStatus::Stopped),
        ]
    }

    proptest! {
        // 속성: 경로가 나오는 것은 요청한 에이전트와 환경이 같은 설치됨 항목이 있을 때뿐이고, 그 경로는 그 항목의 경로다
        #[test]
        fn fr_agent_009_path_comes_only_from_matching_installed_entry(
            entries in prop::collection::vec(
                (agent_strategy(), environment_strategy(), status_strategy()),
                0..8,
            ),
            agent in agent_strategy(),
            environment in environment_strategy(),
        ) {
            // 에이전트와 환경 조합마다 한 줄이라는 감지 결과의 모양을 지킨다
            let mut seen: Vec<(AgentKind, Environment)> = Vec::new();
            let entries: Vec<AgentEntry> = entries
                .into_iter()
                .filter(|(a, e, _)| {
                    if seen.contains(&(*a, e.clone())) {
                        false
                    } else {
                        seen.push((*a, e.clone()));
                        true
                    }
                })
                .map(|(a, e, s)| entry(a, e, s))
                .collect();
            let expected = entries.iter().find_map(|candidate| match &candidate.status {
                AgentStatus::Installed { path, .. }
                    if candidate.agent == agent && candidate.environment == environment =>
                {
                    Some(path.clone())
                }
                _ => None,
            });
            let detection = Detection {
                entries,
                shell_env_failure: None,
            };
            prop_assert_eq!(installed_path(&detection, agent, &environment), expected);
        }
    }
}
