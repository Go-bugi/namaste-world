use super::model::{LoginMethod, LoginStatus};
use serde_json::Value;

/// `claude auth status --json`의 stdout을 해석한다. 종료 코드와 상관없이 JSON의 `loggedIn`으로 판단한다.
/// 로그인됨이면 `authMethod`가 `claude.ai`일 때 구독, `apiProvider`가 `firstParty`가 아니면 클라우드,
/// `authMethod`에 `api`나 `console`이 들어 있으면 API 키, 그 밖은 모르는 방식이다.
/// JSON이 아니거나 `loggedIn`이 bool이 아니면 None.
pub fn claude_status(stdout: &str) -> Option<LoginStatus> {
    let json: Value = serde_json::from_str(stdout.trim()).ok()?;
    if !json.get("loggedIn")?.as_bool()? {
        return Some(LoginStatus::LoggedOut);
    }
    let method = json.get("authMethod").and_then(Value::as_str).map(|auth| {
        let provider = json.get("apiProvider").and_then(Value::as_str);
        if auth == "claude.ai" {
            LoginMethod::Subscription
        } else if provider.is_some_and(|provider| provider != "firstParty") {
            LoginMethod::Cloud
        } else if auth.contains("api") || auth.contains("console") {
            LoginMethod::ApiKey
        } else {
            LoginMethod::Other
        }
    });
    Some(LoginStatus::LoggedIn { method })
}

/// `codex login status`의 stdout과 stderr를 해석한다. 결과 문장은 stderr로 나온다.
/// `Logged in using ChatGPT`는 구독, `Logged in using an API key`는 API 키, 그 밖의 `Logged in using`은 모르는 방식,
/// `Not logged in`은 로그인 안 됨이고 나머지는 None. API 키 뒤에 붙는 가린 키 문자열은 결과에 담지 않는다.
pub fn codex_status(stdout: &str, stderr: &str) -> Option<LoginStatus> {
    stdout
        .lines()
        .chain(stderr.lines())
        .map(str::trim)
        .find_map(|line| {
            if line == "Not logged in" {
                return Some(LoginStatus::LoggedOut);
            }
            // 방식 뒤에 붙는 가린 키는 읽지 않는다
            let method = line.strip_prefix("Logged in using ")?;
            let method = if method == "ChatGPT" {
                LoginMethod::Subscription
            } else if method.starts_with("an API key") {
                LoginMethod::ApiKey
            } else {
                LoginMethod::Other
            };
            Some(LoginStatus::LoggedIn {
                method: Some(method),
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::domain::model::LoginMethod;
    use proptest::prelude::*;

    fn logged_in(method: LoginMethod) -> Option<LoginStatus> {
        Some(LoginStatus::LoggedIn {
            method: Some(method),
        })
    }

    fn claude_json(logged_in: bool, auth_method: &str, api_provider: &str) -> String {
        serde_json::json!({
            "loggedIn": logged_in,
            "authMethod": auth_method,
            "apiProvider": api_provider,
        })
        .to_string()
    }

    #[test]
    fn fr_agent_009_claude_subscription_is_logged_in_with_method() {
        let stdout = claude_json(true, "claude.ai", "firstParty");
        assert_eq!(claude_status(&stdout), logged_in(LoginMethod::Subscription));
    }

    #[test]
    fn fr_agent_009_claude_api_key_is_logged_in() {
        assert_eq!(
            claude_status(&claude_json(true, "api_key", "firstParty")),
            logged_in(LoginMethod::ApiKey)
        );
        assert_eq!(
            claude_status(&claude_json(true, "console", "firstParty")),
            logged_in(LoginMethod::ApiKey)
        );
    }

    #[test]
    fn fr_agent_009_claude_cloud_provider_is_logged_in() {
        // Bedrock 같은 클라우드 제공자로 쓰는 것도 CLI가 알려 주는 로그인이다
        assert_eq!(
            claude_status(&claude_json(true, "third_party", "bedrock")),
            logged_in(LoginMethod::Cloud)
        );
    }

    #[test]
    fn fr_agent_009_claude_unknown_method_is_still_logged_in() {
        assert_eq!(
            claude_status(&claude_json(true, "oauth_token", "firstParty")),
            logged_in(LoginMethod::Other)
        );
    }

    #[test]
    fn fr_agent_009_claude_without_auth_method_is_logged_in_without_method() {
        // CLI가 방식을 알려 주지 않으면 방식 없이 로그인됨이다 (TQ1)
        assert_eq!(
            claude_status(r#"{"loggedIn":true}"#),
            Some(LoginStatus::LoggedIn { method: None })
        );
    }

    #[test]
    fn fr_agent_009_claude_non_string_auth_method_is_logged_in_without_method() {
        // TQ1
        assert_eq!(
            claude_status(r#"{"loggedIn":true,"authMethod":42}"#),
            Some(LoginStatus::LoggedIn { method: None })
        );
        assert_eq!(
            claude_status(r#"{"loggedIn":true,"authMethod":null}"#),
            Some(LoginStatus::LoggedIn { method: None })
        );
    }

    #[test]
    fn fr_agent_009_claude_logged_out() {
        let stdout = r#"{"loggedIn":false}"#;
        assert_eq!(claude_status(stdout), Some(LoginStatus::LoggedOut));
    }

    #[test]
    fn fr_agent_010_claude_non_json_is_unreadable() {
        assert_eq!(claude_status(""), None);
        assert_eq!(claude_status("error: unknown command 'auth'"), None);
    }

    #[test]
    fn fr_agent_010_claude_logged_in_not_bool_is_unreadable() {
        assert_eq!(claude_status(r#"{"loggedIn":"true"}"#), None);
        assert_eq!(claude_status(r#"{"loggedIn":1}"#), None);
        assert_eq!(claude_status(r#"{"authMethod":"claude.ai"}"#), None);
        assert_eq!(claude_status("[]"), None);
    }

    #[test]
    fn fr_agent_009_codex_chatgpt_is_subscription() {
        assert_eq!(
            codex_status("", "Logged in using ChatGPT\n"),
            logged_in(LoginMethod::Subscription)
        );
    }

    #[test]
    fn fr_agent_009_codex_api_key_is_logged_in_without_key() {
        // 결과 타입에 키 문자열이 들어갈 자리가 없고, 방식만 API 키로 담긴다
        assert_eq!(
            codex_status("", "Logged in using an API key - sk-proj-***ABCD\n"),
            logged_in(LoginMethod::ApiKey)
        );
    }

    #[test]
    fn fr_agent_009_codex_unknown_method_is_still_logged_in() {
        assert_eq!(
            codex_status("", "Logged in using a device token\n"),
            logged_in(LoginMethod::Other)
        );
    }

    #[test]
    fn fr_agent_009_codex_not_logged_in() {
        assert_eq!(
            codex_status("", "Not logged in\n"),
            Some(LoginStatus::LoggedOut)
        );
    }

    #[test]
    fn fr_agent_009_codex_reads_stdout_too() {
        // stdout과 stderr를 합쳐 읽으므로 결과 문장이 stdout에 있어도 해석한다
        assert_eq!(
            codex_status("Not logged in\n", ""),
            Some(LoginStatus::LoggedOut)
        );
        assert_eq!(
            codex_status("Logged in using ChatGPT\n", ""),
            logged_in(LoginMethod::Subscription)
        );
    }

    #[test]
    fn fr_agent_010_codex_unrecognized_output_is_unreadable() {
        assert_eq!(codex_status("", ""), None);
        assert_eq!(
            codex_status("", "error: unrecognized subcommand 'status'\n"),
            None
        );
    }

    proptest! {
        // 속성: 로그인 여부는 loggedIn만으로 정해진다. 방식 값이 무엇이든 true면 로그인됨, false면 로그인 안 됨이다
        #[test]
        fn fr_agent_009_claude_logged_in_flag_decides_status(
            flag in any::<bool>(),
            auth_method in ".{0,20}",
            api_provider in ".{0,20}",
        ) {
            let got = claude_status(&claude_json(flag, &auth_method, &api_provider));
            if flag {
                let is_logged_in = matches!(got, Some(LoginStatus::LoggedIn { .. }));
                prop_assert!(is_logged_in);
            } else {
                prop_assert_eq!(got, Some(LoginStatus::LoggedOut));
            }
        }

        // 속성: JSON이 아닌 출력은 로그인 안 됨으로 나오지 않고 해석 실패다
        #[test]
        fn fr_agent_010_claude_non_json_never_logged_out(text in "[a-zA-Z ][a-zA-Z0-9 .:]{0,40}") {
            prop_assert_eq!(claude_status(&text), None);
        }

        // 속성: 가린 키 문자열이 무엇이든 API 키 로그인으로만 해석한다
        #[test]
        fn fr_agent_009_codex_api_key_suffix_is_ignored(suffix in "[ -~]{0,40}") {
            let stderr = format!("Logged in using an API key - {suffix}\n");
            prop_assert_eq!(codex_status("", &stderr), logged_in(LoginMethod::ApiKey));
        }

        // 속성: 결과 문장이 없는 출력은 로그인 안 됨이 아니라 해석 실패다
        #[test]
        fn fr_agent_010_codex_without_status_line_is_unreadable(
            stdout in "[0-9 .:-]{0,40}",
            stderr in "[0-9 .:-]{0,40}",
        ) {
            prop_assert_eq!(codex_status(&stdout, &stderr), None);
        }
    }
}
