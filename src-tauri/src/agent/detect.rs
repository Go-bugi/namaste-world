use super::domain::assemble::{entries_for, should_probe, stopped_entries};
use super::domain::model::{AgentEntry, Detection, FailureReason, Target, Trigger};
use super::host::Host;
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::task::Poll;
use std::time::Duration;
use tokio::time::{timeout_at, Instant};

/// 감지 한 번의 상한 시간.
pub const DETECTION_LIMIT: Duration = Duration::from_secs(10);

/// 모든 환경을 동시에 확인하고 `limit` 안에 결과를 낸다.
/// `should_probe`가 false인 환경은 `probe`를 부르지 않고 `stopped_entries`로 채운다(배포판을 켜지 않음).
/// `limit` 안에 끝나지 않은 환경의 항목은 `Failed(Timeout)`이다. 한 환경의 실패는 다른 환경에 영향이 없다.
/// 항목 순서는 `targets()` 순서, 그 안에서 Claude, Codex 순서다.
pub async fn detect<H: Host>(host: &H, trigger: Trigger, limit: Duration) -> Detection {
    let deadline = Instant::now() + limit;
    let targets = timeout_at(deadline, host.targets())
        .await
        .unwrap_or_default();
    let checks = targets
        .iter()
        .map(|target| check(host, target, trigger, deadline))
        .collect();
    Detection {
        entries: join_all(checks).await.into_iter().flatten().collect(),
    }
}

/// 환경 하나를 마감 시각까지 확인해 항목으로 바꾼다.
async fn check<H: Host>(
    host: &H,
    target: &Target,
    trigger: Trigger,
    deadline: Instant,
) -> Vec<AgentEntry> {
    if !should_probe(target, trigger) {
        return stopped_entries(&target.environment);
    }
    let probe = timeout_at(deadline, host.probe(&target.environment))
        .await
        .unwrap_or(Err(FailureReason::Timeout));
    entries_for(&target.environment, &probe)
}

/// future들을 같은 작업 안에서 동시에 돌리고 입력 순서대로 결과를 모은다.
/// `host`를 빌린 future라 `'static`이 필요한 `spawn`을 쓸 수 없어 직접 폴링한다.
async fn join_all<F: Future>(futures: Vec<F>) -> Vec<F::Output> {
    let mut pending: Vec<Pin<Box<F>>> = futures.into_iter().map(Box::pin).collect();
    let mut outputs: Vec<Option<F::Output>> = pending.iter().map(|_| None).collect();
    poll_fn(|cx| {
        for (future, output) in pending.iter_mut().zip(outputs.iter_mut()) {
            if output.is_none() {
                if let Poll::Ready(value) = future.as_mut().poll(cx) {
                    *output = Some(value);
                }
            }
        }
        if outputs.iter().all(Option::is_some) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
    outputs.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::domain::model::{
        AgentEntry, AgentKind, AgentStatus, EnvProbe, Environment, FailureReason, Found, Target,
    };
    use std::collections::BTreeMap;
    use std::future::Future;
    use std::sync::Mutex;
    use tokio::time::Instant;

    /// 환경 하나에서 프로브가 어떻게 끝나는지.
    #[derive(Clone)]
    enum Outcome {
        /// `after`가 지난 뒤 이 결과를 낸다.
        Returns { after: Duration, probe: EnvProbe },
        /// 끝나지 않는다.
        Hangs,
    }

    struct FakeHost {
        targets: Vec<Target>,
        outcomes: Vec<(Environment, Outcome)>,
        probed: Mutex<Vec<Environment>>,
    }

    impl FakeHost {
        fn new(targets: Vec<Target>, outcomes: Vec<(Environment, Outcome)>) -> Self {
            Self {
                targets,
                outcomes,
                probed: Mutex::new(Vec::new()),
            }
        }

        fn probed(&self) -> Vec<Environment> {
            self.probed.lock().unwrap().clone()
        }
    }

    impl Host for FakeHost {
        fn targets(&self) -> impl Future<Output = Vec<Target>> + Send {
            let targets = self.targets.clone();
            async move { targets }
        }

        fn probe(&self, environment: &Environment) -> impl Future<Output = EnvProbe> + Send {
            self.probed.lock().unwrap().push(environment.clone());
            let outcome = self
                .outcomes
                .iter()
                .find(|(env, _)| env == environment)
                .map(|(_, outcome)| outcome.clone())
                .unwrap_or(Outcome::Returns {
                    after: Duration::ZERO,
                    probe: Ok(BTreeMap::new()),
                });
            async move {
                match outcome {
                    Outcome::Returns { after, probe } => {
                        tokio::time::sleep(after).await;
                        probe
                    }
                    Outcome::Hangs => std::future::pending().await,
                }
            }
        }
    }

    fn ubuntu() -> Environment {
        Environment::Wsl {
            distro: "Ubuntu".to_string(),
        }
    }

    fn debian() -> Environment {
        Environment::Wsl {
            distro: "Debian".to_string(),
        }
    }

    fn running(environment: Environment) -> Target {
        Target {
            environment,
            running: true,
        }
    }

    fn stopped(environment: Environment) -> Target {
        Target {
            environment,
            running: false,
        }
    }

    fn found(path: &str, version_output: &str) -> Found {
        Found {
            path: path.to_string(),
            version_output: Some(version_output.to_string()),
        }
    }

    fn returns(probe: EnvProbe) -> Outcome {
        Outcome::Returns {
            after: Duration::ZERO,
            probe,
        }
    }

    fn codex_only(path: &str) -> EnvProbe {
        Ok(BTreeMap::from([(
            AgentKind::Codex,
            found(path, "codex-cli 0.146.0"),
        )]))
    }

    fn entry(agent: AgentKind, environment: &Environment, status: AgentStatus) -> AgentEntry {
        AgentEntry {
            agent,
            environment: environment.clone(),
            status,
        }
    }

    fn installed(path: &str, version: &str) -> AgentStatus {
        AgentStatus::Installed {
            path: path.to_string(),
            version: version.to_string(),
        }
    }

    fn timeout() -> AgentStatus {
        AgentStatus::Failed {
            reason: FailureReason::Timeout,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_001_both_installed_on_macos() {
        let mac = Environment::Macos;
        let host = FakeHost::new(
            vec![running(mac.clone())],
            vec![(
                mac.clone(),
                returns(Ok(BTreeMap::from([
                    (
                        AgentKind::Claude,
                        found("/Users/u/.local/bin/claude", "2.1.322 (Claude Code)"),
                    ),
                    (
                        AgentKind::Codex,
                        found("/opt/homebrew/bin/codex", "codex-cli 0.146.0"),
                    ),
                ]))),
            )],
        );

        let detection = detect(&host, Trigger::Startup, DETECTION_LIMIT).await;

        assert_eq!(
            detection.entries,
            vec![
                entry(
                    AgentKind::Claude,
                    &mac,
                    installed("/Users/u/.local/bin/claude", "2.1.322")
                ),
                entry(
                    AgentKind::Codex,
                    &mac,
                    installed("/opt/homebrew/bin/codex", "0.146.0")
                ),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_002_same_agent_in_windows_and_wsl_is_two_entries() {
        let win = Environment::Windows;
        let ubu = ubuntu();
        let host = FakeHost::new(
            vec![running(win.clone()), running(ubu.clone())],
            vec![
                (win.clone(), returns(codex_only("C:\\bin\\codex.exe"))),
                (ubu.clone(), returns(codex_only("/usr/local/bin/codex"))),
            ],
        );

        let detection = detect(&host, Trigger::Startup, DETECTION_LIMIT).await;

        assert_eq!(
            detection.entries,
            vec![
                entry(AgentKind::Claude, &win, AgentStatus::NotInstalled),
                entry(
                    AgentKind::Codex,
                    &win,
                    installed("C:\\bin\\codex.exe", "0.146.0")
                ),
                entry(AgentKind::Claude, &ubu, AgentStatus::NotInstalled),
                entry(
                    AgentKind::Codex,
                    &ubu,
                    installed("/usr/local/bin/codex", "0.146.0")
                ),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_002_claude_only_in_one_of_several_distros() {
        let win = Environment::Windows;
        let ubu = ubuntu();
        let deb = debian();
        let host = FakeHost::new(
            vec![
                running(win.clone()),
                running(ubu.clone()),
                running(deb.clone()),
            ],
            vec![(
                deb.clone(),
                returns(Ok(BTreeMap::from([(
                    AgentKind::Claude,
                    found("/home/user/.local/bin/claude", "2.1.322 (Claude Code)"),
                )]))),
            )],
        );

        let detection = detect(&host, Trigger::Startup, DETECTION_LIMIT).await;

        let installed_claude: Vec<&Environment> = detection
            .entries
            .iter()
            .filter(|e| e.agent == AgentKind::Claude)
            .filter(|e| matches!(e.status, AgentStatus::Installed { .. }))
            .map(|e| &e.environment)
            .collect();
        assert_eq!(installed_claude, vec![&deb]);
        assert_eq!(detection.entries.len(), 6);
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_002_startup_does_not_start_stopped_distro() {
        let win = Environment::Windows;
        let ubu = ubuntu();
        let host = FakeHost::new(
            vec![running(win.clone()), stopped(ubu.clone())],
            vec![(ubu.clone(), returns(codex_only("/usr/local/bin/codex")))],
        );

        let detection = detect(&host, Trigger::Startup, DETECTION_LIMIT).await;

        assert_eq!(host.probed(), vec![win.clone()]);
        assert_eq!(
            detection.entries[2..],
            [
                entry(AgentKind::Claude, &ubu, AgentStatus::Stopped),
                entry(AgentKind::Codex, &ubu, AgentStatus::Stopped),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_002_redetect_starts_stopped_distro() {
        let win = Environment::Windows;
        let ubu = ubuntu();
        let host = FakeHost::new(
            vec![running(win.clone()), stopped(ubu.clone())],
            vec![(ubu.clone(), returns(codex_only("/usr/local/bin/codex")))],
        );

        let detection = detect(&host, Trigger::UserRequested, DETECTION_LIMIT).await;

        assert!(host.probed().contains(&ubu));
        assert_eq!(
            detection.entries[3],
            entry(
                AgentKind::Codex,
                &ubu,
                installed("/usr/local/bin/codex", "0.146.0")
            )
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_002_no_wsl_gives_windows_only_without_error() {
        let win = Environment::Windows;
        let host = FakeHost::new(vec![running(win.clone())], vec![]);

        let detection = detect(&host, Trigger::Startup, DETECTION_LIMIT).await;

        assert_eq!(
            detection.entries,
            vec![
                entry(AgentKind::Claude, &win, AgentStatus::NotInstalled),
                entry(AgentKind::Codex, &win, AgentStatus::NotInstalled),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_005_hanging_distro_fails_without_affecting_windows() {
        let win = Environment::Windows;
        let ubu = ubuntu();
        let host = FakeHost::new(
            vec![running(win.clone()), running(ubu.clone())],
            vec![
                (win.clone(), returns(codex_only("C:\\bin\\codex.exe"))),
                (ubu.clone(), Outcome::Hangs),
            ],
        );

        let detection = detect(&host, Trigger::Startup, DETECTION_LIMIT).await;

        assert_eq!(
            detection.entries,
            vec![
                entry(AgentKind::Claude, &win, AgentStatus::NotInstalled),
                entry(
                    AgentKind::Codex,
                    &win,
                    installed("C:\\bin\\codex.exe", "0.146.0")
                ),
                entry(AgentKind::Claude, &ubu, timeout()),
                entry(AgentKind::Codex, &ubu, timeout()),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_005_environment_error_stays_in_that_environment() {
        let win = Environment::Windows;
        let ubu = ubuntu();
        let reason = FailureReason::EnvironmentError {
            message: "wsl.exe exited with 1".to_string(),
        };
        let host = FakeHost::new(
            vec![running(win.clone()), running(ubu.clone())],
            vec![
                (win.clone(), returns(codex_only("C:\\bin\\codex.exe"))),
                (ubu.clone(), returns(Err(reason.clone()))),
            ],
        );

        let detection = detect(&host, Trigger::Startup, DETECTION_LIMIT).await;

        assert_eq!(
            detection.entries[1],
            entry(
                AgentKind::Codex,
                &win,
                installed("C:\\bin\\codex.exe", "0.146.0")
            )
        );
        assert_eq!(
            detection.entries[2..],
            [
                entry(
                    AgentKind::Claude,
                    &ubu,
                    AgentStatus::Failed {
                        reason: reason.clone()
                    }
                ),
                entry(AgentKind::Codex, &ubu, AgentStatus::Failed { reason }),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn sc_agent_001_all_hanging_finishes_within_limit_as_timeout() {
        let win = Environment::Windows;
        let ubu = ubuntu();
        let deb = debian();
        let host = FakeHost::new(
            vec![
                running(win.clone()),
                running(ubu.clone()),
                running(deb.clone()),
            ],
            vec![
                (win.clone(), Outcome::Hangs),
                (ubu.clone(), Outcome::Hangs),
                (deb.clone(), Outcome::Hangs),
            ],
        );

        let started = Instant::now();
        let detection = detect(&host, Trigger::UserRequested, DETECTION_LIMIT).await;

        assert_eq!(DETECTION_LIMIT, Duration::from_secs(10));
        assert!(started.elapsed() <= DETECTION_LIMIT);
        assert_eq!(detection.entries.len(), 6);
        assert!(detection.entries.iter().all(|e| e.status == timeout()));
    }

    #[tokio::test(start_paused = true)]
    async fn sc_agent_001_environments_are_checked_concurrently() {
        // 하나씩 확인하면 6초 + 6초로 상한 10초를 넘어 두 번째 환경이 시간 초과가 된다
        let win = Environment::Windows;
        let ubu = ubuntu();
        let slow = |path: &str| Outcome::Returns {
            after: Duration::from_secs(6),
            probe: codex_only(path),
        };
        let host = FakeHost::new(
            vec![running(win.clone()), running(ubu.clone())],
            vec![
                (win.clone(), slow("C:\\bin\\codex.exe")),
                (ubu.clone(), slow("/usr/local/bin/codex")),
            ],
        );

        let started = Instant::now();
        let detection = detect(&host, Trigger::Startup, DETECTION_LIMIT).await;

        assert!(started.elapsed() <= DETECTION_LIMIT);
        assert_eq!(
            detection.entries[1].status,
            installed("C:\\bin\\codex.exe", "0.146.0")
        );
        assert_eq!(
            detection.entries[3].status,
            installed("/usr/local/bin/codex", "0.146.0")
        );
    }
}
