use super::model::{ModelList, ModelOption, ModelSource, Selection};
use crate::agent::domain::model::AgentKind;

/// Claude Code의 기본 모델 별칭. 에이전트 설정의 기본 모델을 따른다.
const CLAUDE_DEFAULT: &str = "default";

/// 에이전트별 고정 목록. Claude: default, opus, sonnet, fable, haiku. Codex: gpt-5.6-sol, gpt-5.6-terra, gpt-5.6-luna.
/// 별칭이 가리키는 실제 모델을 알 수 없으므로 detail은 모두 None이다.
pub fn fallback_models(agent: AgentKind) -> Vec<ModelOption> {
    let entries: &[(&str, &str)] = match agent {
        AgentKind::Claude => &[
            (CLAUDE_DEFAULT, "Default"),
            ("opus", "Opus"),
            ("sonnet", "Sonnet"),
            ("fable", "Fable"),
            ("haiku", "Haiku"),
        ],
        AgentKind::Codex => &[
            ("gpt-5.6-sol", "GPT-5.6-Sol"),
            ("gpt-5.6-terra", "GPT-5.6-Terra"),
            ("gpt-5.6-luna", "GPT-5.6-Luna"),
        ],
    };
    entries
        .iter()
        .map(|(value, label)| ModelOption {
            value: (*value).to_owned(),
            label: (*label).to_owned(),
            detail: None,
        })
        .collect()
}

/// 조회 결과로 목록과 기본 모델을 정한다.
/// - queried가 None이거나 빈 목록이면 고정 목록을 쓰고 source는 Fallback.
/// - Claude: 기본은 "default". 목록에 없으면 첫 항목.
/// - Codex: 기본은 config_model. 목록에 없으면 label=value인 선택지로 맨 앞에 더한다.
///   config_model이 None이면 첫 항목.
pub fn model_list(
    agent: AgentKind,
    queried: Option<Vec<ModelOption>>,
    config_model: Option<String>,
) -> ModelList {
    let (mut options, source) = match queried {
        Some(options) if !options.is_empty() => (options, ModelSource::Queried),
        _ => (fallback_models(agent), ModelSource::Fallback),
    };
    let preferred = match agent {
        AgentKind::Claude => Some(CLAUDE_DEFAULT.to_owned()),
        AgentKind::Codex => config_model,
    };
    let default_value = match preferred {
        Some(value) if options.iter().any(|option| option.value == value) => value,
        // 사용자가 Codex 설정에 적은 모델은 목록에 없어도 고를 수 있게 맨 앞에 더한다
        Some(value) if agent == AgentKind::Codex => {
            options.insert(
                0,
                ModelOption {
                    value: value.clone(),
                    label: value.clone(),
                    detail: None,
                },
            );
            value
        }
        _ => options[0].value.clone(),
    };
    ModelList {
        options,
        default_value,
        source,
    }
}

/// selection.json 내용을 읽는다. JSON이 아니거나 필드가 모자라면 None.
pub fn parse_selection(contents: &str) -> Option<Selection> {
    serde_json::from_str(contents).ok()
}

/// selection.json에 쓸 내용.
pub fn selection_json(selection: &Selection) -> String {
    serde_json::to_string_pretty(selection).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::domain::model::Environment;
    use crate::model::domain::model::ModelSource;
    use proptest::prelude::*;

    fn option(value: &str) -> ModelOption {
        ModelOption {
            value: value.to_string(),
            label: value.to_uppercase(),
            detail: None,
        }
    }

    fn values(options: &[ModelOption]) -> Vec<&str> {
        options.iter().map(|o| o.value.as_str()).collect()
    }

    fn codex_queried() -> Vec<ModelOption> {
        vec![
            option("gpt-5.6-sol"),
            option("gpt-5.6-terra"),
            option("gpt-5.6-luna"),
        ]
    }

    #[test]
    fn fr_start_007_claude_fallback_is_five_aliases() {
        assert_eq!(
            values(&fallback_models(AgentKind::Claude)),
            vec!["default", "opus", "sonnet", "fable", "haiku"]
        );
    }

    #[test]
    fn fr_start_007_codex_fallback_is_three_list_models() {
        assert_eq!(
            values(&fallback_models(AgentKind::Codex)),
            vec!["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"]
        );
    }

    #[test]
    fn fr_start_004_fallback_options_have_no_detail() {
        for agent in AgentKind::ALL {
            assert!(fallback_models(agent).iter().all(|o| o.detail.is_none()));
        }
    }

    #[test]
    fn fr_start_007_failed_query_uses_fallback() {
        for agent in AgentKind::ALL {
            let got = model_list(agent, None, None);

            assert_eq!(got.source, ModelSource::Fallback);
            assert_eq!(got.options, fallback_models(agent));
        }
    }

    #[test]
    fn fr_start_007_empty_query_uses_fallback() {
        for agent in AgentKind::ALL {
            let got = model_list(agent, Some(vec![]), None);

            assert_eq!(got.source, ModelSource::Fallback);
            assert_eq!(got.options, fallback_models(agent));
        }
    }

    #[test]
    fn fr_start_004_claude_default_is_default_alias() {
        let queried = vec![option("default"), option("opus"), option("sonnet")];

        let got = model_list(AgentKind::Claude, Some(queried.clone()), None);

        assert_eq!(got.source, ModelSource::Queried);
        assert_eq!(got.options, queried);
        assert_eq!(got.default_value, "default");
    }

    #[test]
    fn fr_start_004_claude_fallback_default_is_default_alias() {
        assert_eq!(
            model_list(AgentKind::Claude, None, None).default_value,
            "default"
        );
    }

    #[test]
    fn fr_start_004_claude_without_default_alias_uses_first() {
        let got = model_list(
            AgentKind::Claude,
            Some(vec![option("opus"), option("sonnet")]),
            None,
        );

        assert_eq!(got.default_value, "opus");
    }

    #[test]
    fn fr_start_004_codex_default_is_config_model() {
        let got = model_list(
            AgentKind::Codex,
            Some(codex_queried()),
            Some("gpt-5.6-terra".into()),
        );

        assert_eq!(got.source, ModelSource::Queried);
        assert_eq!(got.options, codex_queried());
        assert_eq!(got.default_value, "gpt-5.6-terra");
    }

    #[test]
    fn fr_start_004_codex_config_model_missing_from_list_is_added_first() {
        let got = model_list(
            AgentKind::Codex,
            Some(codex_queried()),
            Some("gpt-5.5".into()),
        );

        let mut expected = vec![ModelOption {
            value: "gpt-5.5".into(),
            label: "gpt-5.5".into(),
            detail: None,
        }];
        expected.extend(codex_queried());
        assert_eq!(got.options, expected);
        assert_eq!(got.default_value, "gpt-5.5");
    }

    #[test]
    fn fr_start_004_codex_without_config_model_uses_first() {
        let got = model_list(AgentKind::Codex, Some(codex_queried()), None);

        assert_eq!(got.default_value, "gpt-5.6-sol");
    }

    #[test]
    fn fr_start_004_codex_fallback_still_uses_config_model() {
        let got = model_list(AgentKind::Codex, None, Some("gpt-5.6-luna".into()));

        assert_eq!(got.source, ModelSource::Fallback);
        assert_eq!(got.options, fallback_models(AgentKind::Codex));
        assert_eq!(got.default_value, "gpt-5.6-luna");
    }

    fn agent_strategy() -> impl Strategy<Value = AgentKind> {
        prop_oneof![Just(AgentKind::Claude), Just(AgentKind::Codex)]
    }

    proptest! {
        #[test]
        fn fr_start_004_default_value_is_always_an_option(
            agent in agent_strategy(),
            queried in prop::option::of(prop::collection::vec("[a-z0-9.-]{1,12}", 0..6)),
            config in prop::option::of("[a-z0-9.-]{1,12}"),
        ) {
            let queried = queried.map(|vs| vs.iter().map(|v| option(v)).collect());

            let got = model_list(agent, queried, config);

            prop_assert!(got.options.iter().any(|o| o.value == got.default_value));
        }
    }

    fn selection(agent: AgentKind, environment: Environment, model: &str) -> Selection {
        Selection {
            agent,
            environment,
            model: model.to_string(),
        }
    }

    #[test]
    fn fr_start_006_saved_selection_reads_back() {
        let saved = selection(
            AgentKind::Codex,
            Environment::Wsl {
                distro: "Ubuntu".into(),
            },
            "gpt-5.6-sol",
        );

        assert_eq!(parse_selection(&selection_json(&saved)), Some(saved));
    }

    #[test]
    fn fr_start_006_unreadable_selection_file_is_none() {
        assert_eq!(parse_selection(""), None);
        assert_eq!(parse_selection("not json"), None);
        assert_eq!(
            parse_selection(r#"{"agent":"claude","environment":{"kind":"macos"}}"#),
            None
        );
        assert_eq!(
            parse_selection(r#"{"agent":"gemini","environment":{"kind":"macos"},"model":"x"}"#),
            None
        );
    }

    fn environment_strategy() -> impl Strategy<Value = Environment> {
        prop_oneof![
            Just(Environment::Macos),
            Just(Environment::Windows),
            ".{1,16}".prop_map(|distro| Environment::Wsl { distro }),
        ]
    }

    proptest! {
        #[test]
        fn fr_start_006_any_selection_round_trips(
            agent in agent_strategy(),
            environment in environment_strategy(),
            model in ".{0,24}",
        ) {
            let saved = Selection { agent, environment, model };

            prop_assert_eq!(parse_selection(&selection_json(&saved)), Some(saved));
        }
    }
}
