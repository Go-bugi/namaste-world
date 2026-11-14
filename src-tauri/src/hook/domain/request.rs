use super::endpoint::token_matches;
use super::model::HookEvent;
use crate::agent::domain::model::AgentKind;
use serde::de::{self, Deserializer, IgnoredAny, MapAccess, Visitor};
use serde::Deserialize;
use serde_json::Value;
use std::fmt;

/// 이 크기까지는 원래 입력 전체를 넘긴다.
pub const FULL_BODY_LIMIT: usize = 1024 * 1024;

/// 받지 않는 요청.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// 토큰이 없거나 틀리다.
    Unauthorized,
    /// 경로의 에이전트가 `claude`나 `codex`가 아니다.
    NotFound,
}

/// 요청을 받을지 정한다. 토큰을 먼저 보고, 맞으면 경로의 에이전트를 해석한다.
pub fn route(
    agent_segment: &str,
    expected_token: &str,
    given_token: Option<&str>,
) -> Result<AgentKind, Rejection> {
    if !token_matches(expected_token, given_token) {
        return Err(Rejection::Unauthorized);
    }
    AgentKind::ALL
        .into_iter()
        .find(|agent| agent.command() == agent_segment)
        .ok_or(Rejection::NotFound)
}

/// `FULL_BODY_LIMIT` 이하 본문을 해석한다. JSON 객체가 아니면 `None`이다.
///
/// 요약 필드는 `session_id`, `tool_name`, `tool_use_id`, `cwd`, `hook_event_name`과 `is_error`다.
/// `is_error`는 이벤트가 `PostToolUseFailure`면 true, `tool_response`가 객체이고 `is_error`가 bool이면 그 값,
/// `success`가 bool이면 그 반대 값, 그 밖에는 없음이다.
pub fn parse_full(
    agent: AgentKind,
    url_event: &str,
    body: &[u8],
    received_at: u64,
) -> Option<HookEvent> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let object = value.as_object()?;
    let text = |key: &str| object.get(key).and_then(Value::as_str).map(str::to_owned);
    let summary = Summary {
        session_id: text("session_id"),
        tool_name: text("tool_name"),
        tool_use_id: text("tool_use_id"),
        cwd: text("cwd"),
        hook_event_name: text("hook_event_name"),
        tool_response_error: object.get("tool_response").and_then(|response| {
            let flag = |key: &str| response.get(key).and_then(Value::as_bool);
            flag("is_error").or_else(|| flag("success").map(|success| !success))
        }),
    };
    Some(to_event(
        agent,
        url_event,
        summary,
        received_at,
        Some(value),
    ))
}

/// `FULL_BODY_LIMIT`를 넘는 본문을 읽기 도구로 스트림 해석한다. 요약 필드만 담고 `payload: None`,
/// `truncated: true`다. JSON 객체가 아니면 `None`이다.
pub fn parse_summary<R: std::io::Read>(
    agent: AgentKind,
    url_event: &str,
    body: R,
    received_at: u64,
) -> Option<HookEvent> {
    let summary: Summary = serde_json::from_reader(body).ok()?;
    Some(to_event(agent, url_event, summary, received_at, None))
}

/// 본문에서 뽑은 요약 필드. 큰 값(`tool_input`, `tool_response`의 내용 등)은 읽고 버린다.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Summary {
    #[serde(deserialize_with = "text_only")]
    session_id: Option<String>,
    #[serde(deserialize_with = "text_only")]
    tool_name: Option<String>,
    #[serde(deserialize_with = "text_only")]
    tool_use_id: Option<String>,
    #[serde(deserialize_with = "text_only")]
    cwd: Option<String>,
    #[serde(deserialize_with = "text_only")]
    hook_event_name: Option<String>,
    #[serde(rename = "tool_response", deserialize_with = "response_error")]
    tool_response_error: Option<bool>,
}

fn to_event(
    agent: AgentKind,
    url_event: &str,
    summary: Summary,
    received_at: u64,
    payload: Option<Value>,
) -> HookEvent {
    let event = summary
        .hook_event_name
        .unwrap_or_else(|| url_event.to_owned());
    let is_error = if event == "PostToolUseFailure" {
        Some(true)
    } else {
        summary.tool_response_error
    };
    HookEvent {
        agent,
        event,
        received_at,
        session_id: summary.session_id,
        tool_name: summary.tool_name,
        tool_use_id: summary.tool_use_id,
        is_error,
        cwd: summary.cwd,
        truncated: payload.is_none(),
        payload,
    }
}

/// 문자열이면 그 값, 다른 JSON 값이면 읽고 버린 뒤 `None`.
fn text_only<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    struct TextOnly;
    impl<'de> Visitor<'de> for TextOnly {
        type Value = Option<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("any JSON value")
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(Some(v.to_owned()))
        }
        fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
            Ok(Some(v))
        }
        fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            while seq.next_element::<IgnoredAny>()?.is_some() {}
            Ok(None)
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
            Ok(None)
        }
    }
    deserializer.deserialize_any(TextOnly)
}

/// `tool_response`가 객체면 `is_error`, 없으면 `success`의 반대 값. 내용은 메모리에 올리지 않고 읽고 버린다.
fn response_error<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<bool>, D::Error> {
    struct ResponseError;
    impl<'de> Visitor<'de> for ResponseError {
        type Value = Option<bool>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("any JSON value")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut is_error = None;
            let mut success = None;
            while let Some(key) = map.next_key::<String>()? {
                match key.as_str() {
                    "is_error" => is_error = map.next_value::<FlagOnly>()?.0,
                    "success" => success = map.next_value::<FlagOnly>()?.0,
                    _ => {
                        map.next_value::<IgnoredAny>()?;
                    }
                }
            }
            Ok(is_error.or(success.map(|success| !success)))
        }
        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            while seq.next_element::<IgnoredAny>()?.is_some() {}
            Ok(None)
        }
        fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }
    deserializer.deserialize_any(ResponseError)
}

/// bool이면 그 값, 다른 JSON 값이면 읽고 버린 뒤 `None`.
struct FlagOnly(Option<bool>);

impl<'de> Deserialize<'de> for FlagOnly {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = IgnoredOrBool::deserialize(deserializer)?;
        Ok(FlagOnly(match value {
            IgnoredOrBool::Bool(flag) => Some(flag),
            IgnoredOrBool::Other(_) => None,
        }))
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum IgnoredOrBool {
    Bool(bool),
    Other(IgnoredAny),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::io::Cursor;

    const TOKEN: &str = "s3cret-token";
    const AT: u64 = 1_700_000_000_123;

    fn event(agent: AgentKind, name: &str) -> HookEvent {
        HookEvent {
            agent,
            event: name.into(),
            received_at: AT,
            session_id: None,
            tool_name: None,
            tool_use_id: None,
            is_error: None,
            cwd: None,
            payload: None,
            truncated: false,
        }
    }

    fn body(value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }

    fn is_error_of(value: Value) -> Option<bool> {
        parse_full(AgentKind::Claude, "PostToolUse", &body(&value), AT)
            .expect("JSON 객체는 해석해야 한다")
            .is_error
    }

    // route

    #[test]
    fn fr_hook_001_route_accepts_known_agents_with_token() {
        assert_eq!(route("claude", TOKEN, Some(TOKEN)), Ok(AgentKind::Claude));
        assert_eq!(route("codex", TOKEN, Some(TOKEN)), Ok(AgentKind::Codex));
    }

    #[test]
    fn fr_hook_001_route_rejects_missing_or_wrong_token() {
        assert_eq!(route("claude", TOKEN, None), Err(Rejection::Unauthorized));
        assert_eq!(
            route("claude", TOKEN, Some("other-token")),
            Err(Rejection::Unauthorized)
        );
    }

    #[test]
    fn fr_hook_001_route_checks_token_before_agent() {
        assert_eq!(route("gemini", TOKEN, None), Err(Rejection::Unauthorized));
        assert_eq!(
            route("gemini", TOKEN, Some(TOKEN)),
            Err(Rejection::NotFound)
        );
    }

    // parse_full

    #[test]
    fn fr_hook_009_claude_user_prompt_submit_carries_session_and_payload() {
        let input = json!({
            "session_id": "sess-1",
            "transcript_path": "/Users/a/.claude/projects/x.jsonl",
            "cwd": "/Users/a/project",
            "hook_event_name": "UserPromptSubmit",
            "prompt": "안녕"
        });
        let parsed = parse_full(AgentKind::Claude, "UserPromptSubmit", &body(&input), AT);
        assert_eq!(
            parsed,
            Some(HookEvent {
                session_id: Some("sess-1".into()),
                cwd: Some("/Users/a/project".into()),
                payload: Some(input),
                ..event(AgentKind::Claude, "UserPromptSubmit")
            })
        );
    }

    #[test]
    fn fr_hook_009_tool_fields_are_summarized() {
        let input = json!({
            "session_id": "sess-2",
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_use_id": "toolu_01",
            "tool_input": { "command": "ls" }
        });
        let parsed = parse_full(AgentKind::Codex, "PreToolUse", &body(&input), AT);
        assert_eq!(
            parsed,
            Some(HookEvent {
                session_id: Some("sess-2".into()),
                tool_name: Some("Bash".into()),
                tool_use_id: Some("toolu_01".into()),
                payload: Some(input),
                ..event(AgentKind::Codex, "PreToolUse")
            })
        );
    }

    #[test]
    fn fr_hook_009_event_name_prefers_body_then_url() {
        let named = json!({ "hook_event_name": "Stop" });
        assert_eq!(
            parse_full(AgentKind::Claude, "SubagentStop", &body(&named), AT).map(|e| e.event),
            Some("Stop".to_string())
        );
        let unnamed = json!({ "session_id": "s" });
        assert_eq!(
            parse_full(AgentKind::Claude, "SubagentStop", &body(&unnamed), AT).map(|e| e.event),
            Some("SubagentStop".to_string())
        );
    }

    #[test]
    fn fr_hook_009_non_object_body_is_ignored() {
        for raw in [
            &b"not json"[..],
            b"",
            b"[1,2]",
            b"\"text\"",
            b"42",
            b"{\"a\":",
        ] {
            assert_eq!(
                parse_full(AgentKind::Claude, "Stop", raw, AT),
                None,
                "{raw:?}"
            );
        }
    }

    #[test]
    fn fr_hook_009_is_error_from_failure_event() {
        assert_eq!(
            is_error_of(json!({ "hook_event_name": "PostToolUseFailure", "tool_name": "Bash" })),
            Some(true)
        );
    }

    #[test]
    fn fr_hook_009_is_error_from_tool_response() {
        assert_eq!(
            is_error_of(json!({ "tool_response": { "is_error": true } })),
            Some(true)
        );
        assert_eq!(
            is_error_of(json!({ "tool_response": { "is_error": false } })),
            Some(false)
        );
        assert_eq!(
            is_error_of(json!({ "tool_response": { "success": true } })),
            Some(false)
        );
        assert_eq!(
            is_error_of(json!({ "tool_response": { "success": false } })),
            Some(true)
        );
    }

    #[test]
    fn fr_hook_009_is_error_absent_otherwise() {
        assert_eq!(
            is_error_of(json!({ "hook_event_name": "PostToolUse" })),
            None
        );
        assert_eq!(
            is_error_of(json!({ "tool_response": "plain output" })),
            None
        );
        assert_eq!(
            is_error_of(json!({ "tool_response": { "stdout": "ok" } })),
            None
        );
    }

    #[test]
    fn fr_hook_013_200kb_body_keeps_full_payload() {
        let input = json!({
            "session_id": "sess-3",
            "hook_event_name": "PostToolUse",
            "tool_response": "x".repeat(200 * 1024)
        });
        let raw = body(&input);
        assert!(raw.len() <= FULL_BODY_LIMIT);
        let parsed = parse_full(AgentKind::Claude, "PostToolUse", &raw, AT).unwrap();
        assert_eq!(parsed.payload, Some(input));
        assert!(!parsed.truncated);
    }

    // parse_summary

    #[test]
    fn fr_hook_013_large_body_keeps_summary_fields_only() {
        let input = json!({
            "session_id": "sess-4",
            "cwd": "/Users/a/project",
            "hook_event_name": "PostToolUse",
            "tool_name": "Read",
            "tool_input": { "file_path": "/Users/a/project/big.log" },
            "tool_response": { "content": "y".repeat(3 * 1024 * 1024), "is_error": false },
            "tool_use_id": "toolu_02"
        });
        let raw = body(&input);
        assert!(raw.len() > FULL_BODY_LIMIT);
        let parsed = parse_summary(AgentKind::Claude, "PostToolUse", Cursor::new(raw), AT);
        assert_eq!(
            parsed,
            Some(HookEvent {
                session_id: Some("sess-4".into()),
                tool_name: Some("Read".into()),
                tool_use_id: Some("toolu_02".into()),
                is_error: Some(false),
                cwd: Some("/Users/a/project".into()),
                truncated: true,
                ..event(AgentKind::Claude, "PostToolUse")
            })
        );
    }

    #[test]
    fn fr_hook_013_large_non_object_body_is_ignored() {
        let mut raw = vec![b'['; 1];
        raw.extend(std::iter::repeat_n(b'1', 2 * FULL_BODY_LIMIT));
        raw.push(b']');
        assert_eq!(
            parse_summary(AgentKind::Codex, "Stop", Cursor::new(raw), AT),
            None
        );
        let garbage = vec![b'z'; 2 * FULL_BODY_LIMIT];
        assert_eq!(
            parse_summary(AgentKind::Codex, "Stop", Cursor::new(garbage), AT),
            None
        );
    }

    mod props {
        use super::*;
        use proptest::prelude::*;

        fn opt_text() -> impl Strategy<Value = Option<String>> {
            prop::option::of("[a-zA-Z0-9_/ -]{0,16}")
        }

        proptest! {
            #[test]
            fn fr_hook_009_any_bytes_never_panic(raw in prop::collection::vec(any::<u8>(), 0..256)) {
                let _ = parse_full(AgentKind::Claude, "Stop", &raw, AT);
                let _ = parse_summary(AgentKind::Claude, "Stop", Cursor::new(raw), AT);
            }

            #[test]
            fn fr_hook_013_summary_matches_full_without_payload(
                session in opt_text(),
                tool in opt_text(),
                tool_use in opt_text(),
                cwd in opt_text(),
                name in prop::option::of(prop::sample::select(vec!["Stop", "PostToolUse", "PostToolUseFailure"])),
                success in prop::option::of(any::<bool>()),
            ) {
                let mut input = serde_json::Map::new();
                let fields = [("session_id", session), ("tool_name", tool), ("tool_use_id", tool_use), ("cwd", cwd)];
                for (key, value) in fields {
                    if let Some(v) = value {
                        input.insert(key.into(), json!(v));
                    }
                }
                if let Some(n) = name {
                    input.insert("hook_event_name".into(), json!(n));
                }
                if let Some(s) = success {
                    input.insert("tool_response".into(), json!({ "success": s }));
                }
                let raw = serde_json::to_vec(&Value::Object(input)).unwrap();
                let full = parse_full(AgentKind::Codex, "PreToolUse", &raw, AT).expect("객체는 해석해야 한다");
                let summary = parse_summary(AgentKind::Codex, "PreToolUse", Cursor::new(raw), AT);
                prop_assert_eq!(summary, Some(HookEvent { payload: None, truncated: true, ..full }));
            }
        }
    }
}
