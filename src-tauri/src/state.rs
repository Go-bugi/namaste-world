use crate::agent::domain::model::Detection;
use tokio::sync::watch;

/// 마지막으로 끝난 감지 결과. 앱 시작 감지가 끝나기 전에는 `None`이다.
/// 디스크에 저장하지 않으므로 앱을 다시 실행하면 새로 감지한다.
pub struct AgentDetection {
    latest: watch::Sender<Option<Detection>>,
}

impl AgentDetection {
    pub fn new() -> Self {
        Self {
            latest: watch::Sender::new(None),
        }
    }

    /// 보관한 결과. 아직 없으면 첫 결과가 들어올 때까지 기다린다.
    /// 다시 감지하는 중에도 끝나기 전까지는 이전 결과를 바로 준다.
    pub async fn latest(&self) -> Option<Detection> {
        let mut receiver = self.latest.subscribe();
        let result = receiver.wait_for(Option::is_some).await;
        result.ok().and_then(|value| value.clone())
    }

    /// 끝난 감지 결과로 보관한 결과를 바꾼다.
    pub fn publish(&self, detection: Detection) {
        self.latest.send_replace(Some(detection));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::domain::model::{AgentEntry, AgentKind, AgentStatus, Environment};
    use std::time::Duration;
    use tokio::time::timeout;

    fn detection(codex: AgentStatus) -> Detection {
        Detection {
            entries: vec![
                AgentEntry {
                    agent: AgentKind::Claude,
                    environment: Environment::Macos,
                    status: AgentStatus::NotInstalled,
                },
                AgentEntry {
                    agent: AgentKind::Codex,
                    environment: Environment::Macos,
                    status: codex,
                },
            ],
        }
    }

    fn before_install() -> Detection {
        detection(AgentStatus::NotInstalled)
    }

    fn after_install() -> Detection {
        detection(AgentStatus::Installed {
            path: "/opt/homebrew/bin/codex".to_string(),
            version: "0.146.0".to_string(),
        })
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_007_stored_result_is_returned_right_away() {
        let state = AgentDetection::new();
        state.publish(before_install());

        let got = timeout(Duration::ZERO, state.latest()).await;

        assert_eq!(got, Ok(Some(before_install())));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_007_request_before_startup_detection_waits() {
        let state = AgentDetection::new();

        let got = timeout(Duration::from_secs(60), state.latest()).await;

        assert!(got.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_007_waiting_request_gets_startup_result() {
        let state = AgentDetection::new();

        let (got, ()) = tokio::join!(state.latest(), async {
            tokio::time::sleep(Duration::from_secs(3)).await;
            state.publish(before_install());
        });

        assert_eq!(got, Some(before_install()));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_007_redetect_result_replaces_stored_result() {
        let state = AgentDetection::new();
        state.publish(before_install());
        state.publish(after_install());

        assert_eq!(state.latest().await, Some(after_install()));
        assert_eq!(state.latest().await, Some(after_install()));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_agent_007_request_during_redetect_gets_previous_result_right_away() {
        let state = AgentDetection::new();
        state.publish(before_install());

        // 다시 감지는 끝날 때만 publish하므로, 진행 중에는 보관한 결과가 바뀌지 않는다
        let (got, ()) = tokio::join!(timeout(Duration::ZERO, state.latest()), async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            state.publish(after_install());
        });

        assert_eq!(got, Ok(Some(before_install())));
        assert_eq!(state.latest().await, Some(after_install()));
    }
}
