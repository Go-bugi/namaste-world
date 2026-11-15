use super::domain::catalog::model_list;
use super::domain::model::ModelList;
use super::domain::parse::{claude_models, codex_config_model, codex_models};
use super::host::ModelHost;
use crate::agent::domain::model::{AgentKind, Environment};
use std::time::Duration;

/// 모델 목록 조회 한 번의 상한. SC-START-001
pub const MODEL_QUERY_LIMIT: Duration = Duration::from_secs(5);

/// Claude 초기화 요청의 id. 응답 줄 가운데 이 요청의 응답만 고른다.
const CLAUDE_REQUEST_ID: &str = "namaste-models";

/// 한 항목의 모델 목록. `limit` 안에 조회가 끝나지 않으면 고정 목록이다.
/// Codex 설정 파일은 모델 목록 명령과 따로 읽어, 목록 조회가 시간을 넘겨도 설정한 모델을 기본으로 쓴다.
pub async fn query_models(
    host: &impl ModelHost,
    agent: AgentKind,
    environment: &Environment,
    path: &str,
    limit: Duration,
) -> ModelList {
    match agent {
        AgentKind::Claude => {
            let request = serde_json::json!({
                "type": "control_request",
                "request_id": CLAUDE_REQUEST_ID,
                "request": { "subtype": "initialize" },
            })
            .to_string();
            let models = tokio::time::timeout(
                limit,
                host.claude_init(environment, path, &request, |line| {
                    claude_models(line, CLAUDE_REQUEST_ID)
                }),
            )
            .await
            .ok()
            .flatten();
            model_list(agent, models, None)
        }
        AgentKind::Codex => {
            // 두 작업을 함께 시작해 전체가 limit 안에 끝나게 하고, 끝난 쪽의 결과는 각각 쓴다
            let (output, config) = tokio::join!(
                tokio::time::timeout(limit, host.codex_models(environment, path)),
                tokio::time::timeout(limit, host.codex_config(environment)),
            );
            model_list(
                agent,
                output.ok().flatten().as_deref().and_then(codex_models),
                config
                    .ok()
                    .flatten()
                    .as_deref()
                    .and_then(codex_config_model),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::domain::catalog::fallback_models;
    use crate::model::domain::model::ModelSource;
    use serde_json::{json, Value};
    use std::future::Future;
    use std::sync::Mutex;
    use tokio::time::Instant;

    /// 가짜 CLI의 응답 방식.
    #[derive(Clone)]
    enum Reply<T> {
        Value(T),
        Fail,
        Hang,
    }

    struct FakeHost {
        /// 초기화 응답에 넣을 Claude 모델 항목
        claude: Reply<Value>,
        codex_models: Reply<String>,
        codex_config: Option<String>,
        /// 호스트가 받은 (환경, 실행 파일 경로)
        calls: Mutex<Vec<(Environment, String)>>,
    }

    impl FakeHost {
        fn new() -> Self {
            FakeHost {
                claude: Reply::Fail,
                codex_models: Reply::Fail,
                codex_config: None,
                calls: Mutex::new(Vec::new()),
            }
        }

        fn record(&self, environment: &Environment, path: &str) {
            self.calls
                .lock()
                .unwrap()
                .push((environment.clone(), path.to_string()));
        }
    }

    fn claude_lines(request: &str, models: &Value) -> Vec<String> {
        let request: Value = serde_json::from_str(request).expect("초기화 요청은 JSON 한 줄");
        assert_eq!(request["type"], "control_request");
        assert_eq!(request["request"]["subtype"], "initialize");
        let id = request["request_id"]
            .as_str()
            .expect("request_id는 문자열")
            .to_string();
        let response = |request_id: &str| {
            json!({
                "type": "control_response",
                "response": {
                    "subtype": "success",
                    "request_id": request_id,
                    "response": { "commands": [], "models": models }
                }
            })
            .to_string()
        };
        vec![
            json!({ "type": "system", "subtype": "init" }).to_string(),
            response(&format!("{id}-other")),
            response(&id),
        ]
    }

    impl ModelHost for FakeHost {
        fn claude_init<T: Send>(
            &self,
            environment: &Environment,
            path: &str,
            request: &str,
            accept: impl Fn(&str) -> Option<T> + Send + Sync,
        ) -> impl Future<Output = Option<T>> + Send {
            self.record(environment, path);
            let reply = match &self.claude {
                Reply::Value(models) => Reply::Value(claude_lines(request, models)),
                Reply::Fail => Reply::Fail,
                Reply::Hang => Reply::Hang,
            };
            async move {
                match reply {
                    Reply::Value(lines) => lines.iter().find_map(|line| accept(line)),
                    Reply::Fail => None,
                    Reply::Hang => std::future::pending().await,
                }
            }
        }

        fn codex_models(
            &self,
            environment: &Environment,
            path: &str,
        ) -> impl Future<Output = Option<String>> + Send {
            self.record(environment, path);
            let reply = self.codex_models.clone();
            async move {
                match reply {
                    Reply::Value(output) => Some(output),
                    Reply::Fail => None,
                    Reply::Hang => std::future::pending().await,
                }
            }
        }

        fn codex_config(
            &self,
            _environment: &Environment,
        ) -> impl Future<Output = Option<String>> + Send {
            let config = self.codex_config.clone();
            async move { config }
        }
    }

    fn claude_models_json() -> Value {
        json!([
            { "value": "default", "resolvedModel": "claude-opus-5-5", "displayName": "Default (recommended)", "description": "Opus 5.5 · Most capable" },
            { "value": "opus", "resolvedModel": "claude-opus-5-5", "displayName": "Opus", "description": "Opus 5.5" },
            { "value": "sonnet", "resolvedModel": "claude-sonnet-5", "displayName": "Sonnet", "description": "Sonnet 5" },
            { "value": "fable", "resolvedModel": "claude-fable-1", "displayName": "Fable", "description": "Fable 1" },
            { "value": "haiku", "resolvedModel": "claude-haiku-5", "displayName": "Haiku", "description": "Haiku 5" },
            { "value": "claude-opus-4-1", "resolvedModel": "claude-opus-4-1", "displayName": "Opus 4.1", "description": "Legacy" }
        ])
    }

    fn codex_models_json() -> String {
        json!({ "models": [
            { "slug": "gpt-5.6-sol", "display_name": "GPT-5.6-Sol", "description": "desc", "visibility": "list", "priority": 1 },
            { "slug": "gpt-5.6-terra", "display_name": "GPT-5.6-Terra", "description": "desc", "visibility": "list", "priority": 2 },
            { "slug": "gpt-5.6-luna", "display_name": "GPT-5.6-Luna", "description": "desc", "visibility": "list", "priority": 3 },
            { "slug": "hidden-a", "display_name": "Hidden A", "visibility": "hide", "priority": 4 },
            { "slug": "hidden-b", "display_name": "Hidden B", "visibility": "hide", "priority": 5 }
        ]})
        .to_string()
    }

    fn values(list: &ModelList) -> Vec<&str> {
        list.options.iter().map(|o| o.value.as_str()).collect()
    }

    #[tokio::test(start_paused = true)]
    async fn fr_start_007_claude_models_come_from_init_response() {
        let host = FakeHost {
            claude: Reply::Value(claude_models_json()),
            ..FakeHost::new()
        };

        let got = query_models(
            &host,
            AgentKind::Claude,
            &Environment::Macos,
            "/opt/homebrew/bin/claude",
            MODEL_QUERY_LIMIT,
        )
        .await;

        assert_eq!(got.source, ModelSource::Queried);
        assert_eq!(
            values(&got),
            vec![
                "default",
                "opus",
                "sonnet",
                "fable",
                "haiku",
                "claude-opus-4-1"
            ]
        );
        assert_eq!(got.default_value, "default");
        // 별칭에만 실제 모델 ID가 붙고 설명문은 없다(FR-START-004)
        assert_eq!(
            got.options
                .iter()
                .map(|o| o.detail.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("claude-opus-5-5"),
                Some("claude-opus-5-5"),
                Some("claude-sonnet-5"),
                Some("claude-fable-1"),
                Some("claude-haiku-5"),
                None,
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fr_start_007_query_runs_detected_path_in_its_environment() {
        let host = FakeHost {
            codex_models: Reply::Value(codex_models_json()),
            ..FakeHost::new()
        };
        let ubuntu = Environment::Wsl {
            distro: "Ubuntu".into(),
        };

        query_models(
            &host,
            AgentKind::Codex,
            &ubuntu,
            "/home/u/.local/bin/codex",
            MODEL_QUERY_LIMIT,
        )
        .await;

        assert_eq!(
            *host.calls.lock().unwrap(),
            vec![(ubuntu, "/home/u/.local/bin/codex".to_string())]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fr_start_007_codex_shows_only_list_models() {
        let host = FakeHost {
            codex_models: Reply::Value(codex_models_json()),
            ..FakeHost::new()
        };

        let got = query_models(
            &host,
            AgentKind::Codex,
            &Environment::Macos,
            "/usr/local/bin/codex",
            MODEL_QUERY_LIMIT,
        )
        .await;

        assert_eq!(got.source, ModelSource::Queried);
        assert_eq!(
            values(&got),
            vec!["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"]
        );
        assert_eq!(got.default_value, "gpt-5.6-sol");
        assert!(got.options.iter().all(|o| o.detail.is_none()));
    }

    #[tokio::test(start_paused = true)]
    async fn fr_start_004_codex_default_comes_from_config_toml() {
        let host = FakeHost {
            codex_models: Reply::Value(codex_models_json()),
            codex_config: Some("model = \"gpt-5.6-terra\"\n".into()),
            ..FakeHost::new()
        };

        let got = query_models(
            &host,
            AgentKind::Codex,
            &Environment::Windows,
            "C:\\Users\\u\\AppData\\Roaming\\npm\\codex.cmd",
            MODEL_QUERY_LIMIT,
        )
        .await;

        assert_eq!(got.default_value, "gpt-5.6-terra");
        assert_eq!(
            values(&got),
            vec!["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fr_start_004_codex_config_model_missing_from_list_is_added_and_default() {
        let host = FakeHost {
            codex_models: Reply::Value(codex_models_json()),
            codex_config: Some("model = \"gpt-5.5\"\n".into()),
            ..FakeHost::new()
        };

        let got = query_models(
            &host,
            AgentKind::Codex,
            &Environment::Macos,
            "/usr/local/bin/codex",
            MODEL_QUERY_LIMIT,
        )
        .await;

        assert_eq!(
            values(&got),
            vec!["gpt-5.5", "gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"]
        );
        assert_eq!(got.default_value, "gpt-5.5");
    }

    #[tokio::test(start_paused = true)]
    async fn fr_start_007_failed_cli_uses_fallback() {
        let host = FakeHost::new();

        for agent in AgentKind::ALL {
            let got = query_models(
                &host,
                agent,
                &Environment::Macos,
                "/bin/x",
                MODEL_QUERY_LIMIT,
            )
            .await;

            assert_eq!(got.source, ModelSource::Fallback);
            assert_eq!(got.options, fallback_models(agent));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn fr_start_007_unparsable_codex_output_uses_fallback() {
        let host = FakeHost {
            codex_models: Reply::Value("error: not logged in".into()),
            ..FakeHost::new()
        };

        let got = query_models(
            &host,
            AgentKind::Codex,
            &Environment::Macos,
            "/bin/codex",
            MODEL_QUERY_LIMIT,
        )
        .await;

        assert_eq!(got.source, ModelSource::Fallback);
        assert_eq!(got.options, fallback_models(AgentKind::Codex));
    }

    #[tokio::test(start_paused = true)]
    async fn sc_start_001_hanging_claude_falls_back_within_limit() {
        let host = FakeHost {
            claude: Reply::Hang,
            ..FakeHost::new()
        };
        let started = Instant::now();

        let got = query_models(
            &host,
            AgentKind::Claude,
            &Environment::Macos,
            "/bin/claude",
            MODEL_QUERY_LIMIT,
        )
        .await;

        assert!(started.elapsed() <= Duration::from_secs(5));
        assert_eq!(got.source, ModelSource::Fallback);
        assert_eq!(got.options, fallback_models(AgentKind::Claude));
        assert_eq!(got.default_value, "default");
    }

    #[tokio::test(start_paused = true)]
    async fn sc_start_001_hanging_codex_falls_back_within_limit() {
        let host = FakeHost {
            codex_models: Reply::Hang,
            ..FakeHost::new()
        };
        let started = Instant::now();

        let got = query_models(
            &host,
            AgentKind::Codex,
            &Environment::Macos,
            "/bin/codex",
            MODEL_QUERY_LIMIT,
        )
        .await;

        assert!(started.elapsed() <= Duration::from_secs(5));
        assert_eq!(got.source, ModelSource::Fallback);
        assert_eq!(got.options, fallback_models(AgentKind::Codex));
    }
}
