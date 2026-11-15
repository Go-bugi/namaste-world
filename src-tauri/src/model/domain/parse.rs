use super::model::ModelOption;
use serde_json::Value;

/// Claude Code stream-json 출력 한 줄이 초기화 응답이면 그 안의 모델 목록을 꺼낸다.
/// 응답 모양: `{"type":"control_response","response":{"subtype":"success","request_id":<id>,"response":{"models":[...]}}}`.
/// `request_id`가 다르거나 subtype이 success가 아니거나 초기화 응답이 아니면 None.
/// 모델 항목은 value→value, displayName→label을 읽고 둘 중 하나라도 없는 항목은 건너뛴다.
/// resolvedModel이 있고 value와 다르면 detail에 넣는다(별칭). description은 읽지 않는다. 응답 안의 순서를 그대로 둔다.
pub fn claude_models(line: &str, request_id: &str) -> Option<Vec<ModelOption>> {
    let message: Value = serde_json::from_str(line).ok()?;
    if message.get("type")?.as_str()? != "control_response" {
        return None;
    }
    let response = message.get("response")?;
    if response.get("subtype")?.as_str()? != "success"
        || response.get("request_id")?.as_str()? != request_id
    {
        return None;
    }
    let models = response.get("response")?.get("models")?.as_array()?;
    Some(models.iter().filter_map(claude_option).collect())
}

fn claude_option(entry: &Value) -> Option<ModelOption> {
    let value = entry.get("value")?.as_str()?;
    let label = entry.get("displayName")?.as_str()?;
    let detail = entry
        .get("resolvedModel")
        .and_then(Value::as_str)
        .filter(|resolved| *resolved != value)
        .map(str::to_owned);
    Some(ModelOption {
        value: value.to_owned(),
        label: label.to_owned(),
        detail,
    })
}

/// `codex debug models` 출력에서 visibility가 "list"인 모델만 priority 오름차순으로 꺼낸다.
/// JSON이 아니거나 `models` 배열이 없으면 None. value는 slug, label은 display_name(없으면 slug), detail은 None.
pub fn codex_models(output: &str) -> Option<Vec<ModelOption>> {
    let parsed: Value = serde_json::from_str(output).ok()?;
    let models = parsed.get("models")?.as_array()?;
    let mut listed: Vec<(i64, ModelOption)> = models
        .iter()
        .filter(|entry| entry.get("visibility").and_then(Value::as_str) == Some("list"))
        .filter_map(|entry| {
            let slug = entry.get("slug")?.as_str()?;
            let label = entry
                .get("display_name")
                .and_then(Value::as_str)
                .unwrap_or(slug);
            let priority = entry
                .get("priority")
                .and_then(Value::as_i64)
                .unwrap_or(i64::MAX);
            Some((
                priority,
                ModelOption {
                    value: slug.to_owned(),
                    label: label.to_owned(),
                    detail: None,
                },
            ))
        })
        .collect();
    // 같은 priority는 출력 순서를 지킨다
    listed.sort_by_key(|(priority, _)| *priority);
    Some(listed.into_iter().map(|(_, option)| option).collect())
}

/// Codex config.toml의 최상위 `model` 문자열. 파일이 TOML이 아니거나 값이 없거나 빈 문자열이면 None.
pub fn codex_config_model(config: &str) -> Option<String> {
    let document: toml_edit::DocumentMut = config.parse().ok()?;
    document
        .get("model")?
        .as_str()
        .filter(|model| !model.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::{json, Value};

    fn option(value: &str, label: &str, detail: Option<&str>) -> ModelOption {
        ModelOption {
            value: value.to_string(),
            label: label.to_string(),
            detail: detail.map(str::to_string),
        }
    }

    fn init_response(request_id: &str, models: Value) -> String {
        json!({
            "type": "control_response",
            "response": {
                "subtype": "success",
                "request_id": request_id,
                "response": {
                    "commands": [],
                    "models": models,
                    "account": {}
                }
            }
        })
        .to_string()
    }

    fn claude_entries() -> Value {
        json!([
            { "value": "default", "resolvedModel": "claude-opus-5-5", "displayName": "Default (recommended)", "description": "Opus 5.5 · Most capable" },
            { "value": "opus", "resolvedModel": "claude-opus-5-5", "displayName": "Opus", "description": "Opus 5.5" },
            { "value": "sonnet", "resolvedModel": "claude-sonnet-5", "displayName": "Sonnet", "description": "Sonnet 5" },
            { "value": "fable", "resolvedModel": "claude-fable-1", "displayName": "Fable", "description": "Fable 1" },
            { "value": "haiku", "resolvedModel": "claude-haiku-5", "displayName": "Haiku", "description": "Haiku 5" },
            { "value": "claude-opus-4-1", "resolvedModel": "claude-opus-4-1", "displayName": "Opus 4.1", "description": "Legacy" },
            { "value": "claude-opus-4", "displayName": "Opus 4", "description": "Legacy" }
        ])
    }

    #[test]
    fn fr_start_007_claude_init_response_models_in_response_order() {
        let line = init_response("req-1", claude_entries());

        let got = claude_models(&line, "req-1").expect("초기화 응답이면 Some");

        let pairs: Vec<(&str, &str)> = got
            .iter()
            .map(|o| (o.value.as_str(), o.label.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("default", "Default (recommended)"),
                ("opus", "Opus"),
                ("sonnet", "Sonnet"),
                ("fable", "Fable"),
                ("haiku", "Haiku"),
                ("claude-opus-4-1", "Opus 4.1"),
                ("claude-opus-4", "Opus 4"),
            ]
        );
    }

    #[test]
    fn fr_start_004_claude_alias_detail_is_resolved_model_without_description() {
        let line = init_response("req-1", claude_entries());

        let got = claude_models(&line, "req-1");

        assert_eq!(
            got,
            Some(vec![
                option("default", "Default (recommended)", Some("claude-opus-5-5")),
                option("opus", "Opus", Some("claude-opus-5-5")),
                option("sonnet", "Sonnet", Some("claude-sonnet-5")),
                option("fable", "Fable", Some("claude-fable-1")),
                option("haiku", "Haiku", Some("claude-haiku-5")),
                // value와 실제 모델 ID가 같거나 resolvedModel이 없으면 별칭이 아니다
                option("claude-opus-4-1", "Opus 4.1", None),
                option("claude-opus-4", "Opus 4", None),
            ])
        );
    }

    #[test]
    fn fr_start_007_claude_error_response_is_none() {
        let line = json!({
            "type": "control_response",
            "response": {
                "subtype": "error",
                "request_id": "req-1",
                "error": "Unsupported control request subtype: initialize"
            }
        })
        .to_string();

        assert_eq!(claude_models(&line, "req-1"), None);
    }

    #[test]
    fn fr_start_007_claude_other_request_id_is_none() {
        let line = init_response("req-1", claude_entries());

        assert_eq!(claude_models(&line, "req-2"), None);
    }

    #[test]
    fn fr_start_007_claude_non_init_lines_are_none() {
        let system = json!({ "type": "system", "subtype": "init", "session_id": "s" }).to_string();

        assert_eq!(claude_models(&system, "req-1"), None);
        assert_eq!(claude_models("not json", "req-1"), None);
        assert_eq!(claude_models("", "req-1"), None);
    }

    #[test]
    fn fr_start_007_claude_skips_entries_without_value_or_display_name() {
        let line = init_response(
            "req-1",
            json!([
                { "displayName": "No value", "description": "x" },
                { "value": "opus", "displayName": "Opus" },
                { "value": "no-label", "description": "x" },
                { "value": "haiku", "resolvedModel": "claude-haiku-5", "displayName": "Haiku", "description": "Haiku 5" }
            ]),
        );

        assert_eq!(
            claude_models(&line, "req-1"),
            Some(vec![
                option("opus", "Opus", None),
                option("haiku", "Haiku", Some("claude-haiku-5")),
            ])
        );
    }

    fn codex_output(models: Value) -> String {
        json!({ "models": models }).to_string()
    }

    #[test]
    fn fr_start_007_codex_keeps_only_list_models_by_priority() {
        let output = codex_output(json!([
            { "slug": "gpt-5.6-luna", "display_name": "GPT-5.6-Luna", "description": "Fast", "visibility": "list", "priority": 3 },
            { "slug": "codex-internal", "display_name": "Internal", "visibility": "hide", "priority": 0 },
            { "slug": "gpt-5.6-sol", "display_name": "GPT-5.6-Sol", "description": "Most capable", "visibility": "list", "priority": 1 },
            { "slug": "old-model", "display_name": "Old", "visibility": "hide", "priority": 2 },
            { "slug": "gpt-5.6-terra", "display_name": "GPT-5.6-Terra", "visibility": "list", "priority": 2 }
        ]));

        let got = codex_models(&output).expect("models 배열이 있으면 Some");

        let pairs: Vec<(&str, &str)> = got
            .iter()
            .map(|o| (o.value.as_str(), o.label.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("gpt-5.6-sol", "GPT-5.6-Sol"),
                ("gpt-5.6-terra", "GPT-5.6-Terra"),
                ("gpt-5.6-luna", "GPT-5.6-Luna"),
            ]
        );
    }

    #[test]
    fn fr_start_004_codex_models_have_no_detail() {
        let output = codex_output(json!([
            { "slug": "gpt-5.6-sol", "display_name": "GPT-5.6-Sol", "description": "Most capable", "visibility": "list", "priority": 1 },
            { "slug": "gpt-5.6-terra", "display_name": "GPT-5.6-Terra", "description": "Balanced", "visibility": "list", "priority": 2 }
        ]));

        let got = codex_models(&output).expect("models 배열이 있으면 Some");

        assert_eq!(
            got,
            vec![
                option("gpt-5.6-sol", "GPT-5.6-Sol", None),
                option("gpt-5.6-terra", "GPT-5.6-Terra", None),
            ]
        );
    }

    #[test]
    fn fr_start_007_codex_label_falls_back_to_slug() {
        let output = codex_output(json!([
            { "slug": "gpt-5.6-sol", "visibility": "list", "priority": 1 }
        ]));

        let got = codex_models(&output).expect("models 배열이 있으면 Some");

        assert_eq!(got.len(), 1);
        assert_eq!(got[0].value, "gpt-5.6-sol");
        assert_eq!(got[0].label, "gpt-5.6-sol");
    }

    #[test]
    fn fr_start_007_codex_unreadable_output_is_none() {
        assert_eq!(codex_models("error: not logged in"), None);
        assert_eq!(codex_models(""), None);
        assert_eq!(codex_models(&json!({ "data": [] }).to_string()), None);
        assert_eq!(codex_models(&json!({ "models": "x" }).to_string()), None);
    }

    proptest! {
        #[test]
        fn fr_start_007_codex_result_is_list_models_sorted_by_priority(
            models in prop::collection::vec((any::<bool>(), 0i64..100), 0..12)
        ) {
            let entries: Vec<Value> = models
                .iter()
                .enumerate()
                .map(|(i, (listed, priority))| json!({
                    "slug": format!("m{i}"),
                    "display_name": format!("M{i}"),
                    "visibility": if *listed { "list" } else { "hide" },
                    "priority": priority
                }))
                .collect();
            let got = codex_models(&codex_output(Value::Array(entries))).expect("Some");

            // 결과는 목록 표시 모델만, priority가 줄지 않는 순서다
            let listed = models.iter().filter(|(listed, _)| *listed).count();
            prop_assert_eq!(got.len(), listed);
            let priorities: Vec<i64> = got
                .iter()
                .map(|o| {
                    let i: usize = o.value[1..].parse().expect("slug는 m<번호>");
                    prop_assert!(models[i].0);
                    Ok(models[i].1)
                })
                .collect::<Result<_, TestCaseError>>()?;
            prop_assert!(priorities.windows(2).all(|w| w[0] <= w[1]));
        }
    }

    #[test]
    fn fr_start_004_codex_config_top_level_model() {
        let config =
            "model = \"gpt-5.5\"\napproval_policy = \"never\"\n\n[features]\nhooks = true\n";

        assert_eq!(codex_config_model(config), Some("gpt-5.5".to_string()));
    }

    #[test]
    fn fr_start_004_codex_config_model_only_in_table_is_none() {
        let config = "[profiles.work]\nmodel = \"gpt-5.6-sol\"\n";

        assert_eq!(codex_config_model(config), None);
    }

    #[test]
    fn fr_start_004_codex_config_without_usable_model_is_none() {
        assert_eq!(codex_config_model(""), None);
        assert_eq!(codex_config_model("model = \"\"\n"), None);
        assert_eq!(codex_config_model("model = = broken"), None);
        assert_eq!(codex_config_model("approval_policy = \"never\"\n"), None);
    }
}
