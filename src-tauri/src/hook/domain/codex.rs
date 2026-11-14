use super::hooks_json::merge_hook_groups;
use super::model::{Merge, MergeError};
use serde_json::Value;
use sha2::{Digest, Sha256};
use toml_edit::{value, DocumentMut, Item, Table, TableLike};

/// 앱 훅 group이 `hooks.json`에서 놓인 곳. 신뢰 상태 키를 만드는 데 쓴다.
#[derive(Debug, Clone, PartialEq)]
pub struct CodexHookSlot {
    /// 이벤트 이름(PascalCase).
    pub event: String,
    /// 그 이벤트 배열 안에서 앱 group의 위치.
    pub group_index: usize,
    pub handler: Value,
}

/// Codex가 handler에 `timeout`이 없을 때 쓰는 값(초).
const CODEX_DEFAULT_TIMEOUT: u64 = 600;

/// `~/.codex/hooks.json`에 앱 훅을 병합한다. 규칙은 `merge_claude_settings`와 같고, 앱 group의 위치도 돌려준다.
pub fn merge_codex_hooks(
    existing: Option<&str>,
    desired: &[(&str, Value)],
) -> Result<(Merge, Vec<CodexHookSlot>), MergeError> {
    let (merge, placed) = merge_hook_groups(existing, desired)?;
    let slots = placed
        .into_iter()
        .map(|(event, group_index, handler)| CodexHookSlot {
            event,
            group_index,
            handler,
        })
        .collect();
    Ok((merge, slots))
}

/// Codex 훅 신뢰 해시. `{event_name, hooks:[{type, command, timeout, async}]}`를 키 정렬한 compact JSON으로
/// 만들어 SHA-256을 구하고 `"sha256:<소문자 hex>"`로 돌려준다. `matcher`는 넣지 않는다.
///
/// Codex 0.146.0 `hooks/src/engine/discovery.rs`의 `command_hook_hash`와 같은 계산이다.
pub fn codex_hook_hash(event: &str, handler: &Value) -> String {
    let command = handler
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let timeout = handler
        .get("timeout")
        .and_then(Value::as_u64)
        .unwrap_or(CODEX_DEFAULT_TIMEOUT)
        .max(1);
    let is_async = handler
        .get("async")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    // 키를 사전순으로 직접 늘어놓아 serde_json의 키 순서 설정과 상관없이 같은 바이트를 만든다
    let identity = format!(
        r#"{{"event_name":{event},"hooks":[{{"async":{is_async},"command":{command},"timeout":{timeout},"type":"command"}}]}}"#,
        event = Value::from(snake_case(event)),
        command = Value::from(command),
    );
    let digest = Sha256::digest(identity.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{hex}")
}

/// `PreToolUse` → `pre_tool_use`
fn snake_case(event: &str) -> String {
    event
        .chars()
        .enumerate()
        .fold(String::new(), |mut out, (index, c)| {
            if c.is_ascii_uppercase() && index > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            out
        })
}

/// `~/.codex/config.toml`에 훅 기능과 신뢰 상태를 병합한다.
///
/// `[features] hooks`(없으면 예전 이름 `codex_hooks`)가 false면 `HooksDisabled`, 둘 다 없으면 `hooks = true`를 넣는다.
/// slot마다 `[hooks.state."<hooks_json_path>:<snake_case 이벤트>:<group_index>:0"]`에 `trusted_hash`를 넣고
/// `enabled`는 건드리지 않는다. 다른 키, 주석, 서식은 그대로다.
pub fn merge_codex_config(
    existing: Option<&str>,
    hooks_json_path: &str,
    slots: &[CodexHookSlot],
) -> Result<Merge, MergeError> {
    let mut doc: DocumentMut = existing
        .unwrap_or_default()
        .parse()
        .map_err(|_| MergeError::InvalidToml)?;
    let mut changed = false;

    let features = child_table(doc.as_table_mut(), "features", false)?;
    let flag = |key: &str| features.get(key).and_then(Item::as_bool);
    match flag("hooks").or_else(|| flag("codex_hooks")) {
        Some(false) => return Err(MergeError::HooksDisabled),
        Some(true) => {}
        None => {
            features.insert("hooks", value(true));
            changed = true;
        }
    }

    let hooks = child_table(doc.as_table_mut(), "hooks", true)?;
    let state = child_table(hooks, "state", true)?;
    for slot in slots {
        let key = format!(
            "{hooks_json_path}:{}:{}:0",
            snake_case(&slot.event),
            slot.group_index
        );
        let hash = codex_hook_hash(&slot.event, &slot.handler);
        let entry = child_table(state, &key, false)?;
        if entry.get("trusted_hash").and_then(Item::as_str) != Some(hash.as_str()) {
            entry.insert("trusted_hash", value(hash));
            changed = true;
        }
    }

    // 새로 만든 빈 표는 출력에 나오지 않으므로 바뀐 것이 없으면 그대로 둔다
    Ok(if changed {
        Merge::Updated(doc.to_string())
    } else {
        Merge::Unchanged
    })
}

/// 하위 표. 없으면 만든다. `implicit`이면 자기 머리글(`[hooks]`)을 쓰지 않고 하위 표의 머리글만 쓴다.
fn child_table<'a>(
    parent: &'a mut dyn TableLike,
    key: &str,
    implicit: bool,
) -> Result<&'a mut dyn TableLike, MergeError> {
    parent
        .entry(key)
        .or_insert_with(|| {
            let mut table = Table::new();
            table.set_implicit(implicit);
            Item::Table(table)
        })
        .as_table_like_mut()
        .ok_or(MergeError::NotObject)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    /// `{"event_name":"pre_tool_use","hooks":[{"async":false,"command":"echo hi","timeout":600,"type":"command"}]}`의 해시.
    const ECHO_HI_PRE_TOOL_USE_HASH: &str =
        "sha256:c2ab198fe0750494126041254c4d031ad06f4b267edca02168d8a6745e4141c5";
    /// `{"event_name":"user_prompt_submit","hooks":[{"async":false,"command":"x","timeout":5,"type":"command"}]}`의 해시.
    const X_USER_PROMPT_SUBMIT_HASH: &str =
        "sha256:c598e2bef318d295960a7ad04172b410022629cc354ae6d53a35dd88ee08006f";
    const HOOKS_JSON: &str = "/Users/a/.codex/hooks.json";

    fn echo_hi() -> Value {
        json!({ "type": "command", "command": "echo hi", "timeout": 600 })
    }

    fn app_handler(event: &str, version: u32) -> Value {
        json!({
            "type": "command",
            "command": format!(": namaste-world-hook; v{version} {event}"),
            "timeout": 5
        })
    }

    fn desired(events: &[&'static str], version: u32) -> Vec<(&'static str, Value)> {
        events
            .iter()
            .map(|event| (*event, app_handler(event, version)))
            .collect()
    }

    fn app_group(event: &str, version: u32) -> Value {
        json!({ "hooks": [app_handler(event, version)] })
    }

    fn user_group() -> Value {
        json!({ "hooks": [{ "type": "command", "command": "notify-send codex" }] })
    }

    fn orca_group() -> Value {
        json!({ "hooks": [{ "type": "command", "command": "orca-hook codex", "timeout": 10 }] })
    }

    fn slot(event: &str, group_index: usize, handler: Value) -> CodexHookSlot {
        CodexHookSlot {
            event: event.into(),
            group_index,
            handler,
        }
    }

    fn sorted_slots(mut slots: Vec<CodexHookSlot>) -> Vec<CodexHookSlot> {
        slots.sort_by(|a, b| a.event.cmp(&b.event));
        slots
    }

    fn updated_json(merge: Merge) -> Value {
        match merge {
            Merge::Updated(text) => serde_json::from_str(&text).expect("결과가 JSON이어야 한다"),
            Merge::Unchanged => panic!("Updated가 아니다"),
        }
    }

    fn updated_toml(result: Result<Merge, MergeError>) -> (String, toml_edit::DocumentMut) {
        match result {
            Ok(Merge::Updated(text)) => {
                let doc = text.parse().expect("결과가 TOML이어야 한다");
                (text, doc)
            }
            other => panic!("Updated가 아니다: {other:?}"),
        }
    }

    fn state_key(event_snake: &str, index: usize) -> String {
        format!("{HOOKS_JSON}:{event_snake}:{index}:0")
    }

    fn trusted_hash<'a>(doc: &'a toml_edit::DocumentMut, key: &str) -> Option<&'a str> {
        doc.get("hooks")?
            .get("state")?
            .get(key)?
            .get("trusted_hash")?
            .as_str()
    }

    // merge_codex_hooks

    #[test]
    fn fr_hook_002_codex_first_install_registers_each_event_at_index_0() {
        let want = desired(&["PreToolUse", "Stop"], 1);
        let (merge, slots) = merge_codex_hooks(None, &want).unwrap();
        let merged = updated_json(merge);
        assert_eq!(
            merged["hooks"]["PreToolUse"],
            json!([app_group("PreToolUse", 1)])
        );
        assert_eq!(merged["hooks"]["Stop"], json!([app_group("Stop", 1)]));
        assert_eq!(
            sorted_slots(slots),
            vec![
                slot("PreToolUse", 0, app_handler("PreToolUse", 1)),
                slot("Stop", 0, app_handler("Stop", 1)),
            ]
        );
    }

    #[test]
    fn fr_hook_004_codex_keeps_other_groups_and_reports_app_group_position() {
        let existing = json!({
            "hooks": { "Stop": [app_group("Stop", 1), user_group(), user_group()] }
        })
        .to_string();
        let (merge, slots) = merge_codex_hooks(Some(&existing), &desired(&["Stop"], 2)).unwrap();
        assert_eq!(
            updated_json(merge)["hooks"]["Stop"],
            json!([app_group("Stop", 2), user_group(), user_group()])
        );
        assert_eq!(slots, vec![slot("Stop", 0, app_handler("Stop", 2))]);
    }

    #[test]
    fn fr_hook_004_codex_app_group_before_other_tool_is_not_moved() {
        let existing = json!({
            "hooks": { "Stop": [app_group("Stop", 1), orca_group()] }
        })
        .to_string();
        let (merge, slots) = merge_codex_hooks(Some(&existing), &desired(&["Stop"], 1)).unwrap();
        assert_eq!(merge, Merge::Unchanged);
        assert_eq!(slots, vec![slot("Stop", 0, app_handler("Stop", 1))]);
    }

    #[test]
    fn fr_hook_004_codex_app_group_after_other_tool_stays_after_it() {
        let existing = json!({
            "hooks": { "Stop": [orca_group(), app_group("Stop", 1)] }
        })
        .to_string();
        let (merge, slots) = merge_codex_hooks(Some(&existing), &desired(&["Stop"], 2)).unwrap();
        assert_eq!(
            updated_json(merge)["hooks"]["Stop"],
            json!([orca_group(), app_group("Stop", 2)])
        );
        assert_eq!(slots, vec![slot("Stop", 1, app_handler("Stop", 2))]);
    }

    #[test]
    fn fr_hook_002_codex_duplicate_app_groups_collapse_into_first_position() {
        let existing = json!({
            "hooks": {
                "Stop": [user_group(), app_group("Stop", 1), orca_group(), app_group("Stop", 1)]
            }
        })
        .to_string();
        let (merge, slots) = merge_codex_hooks(Some(&existing), &desired(&["Stop"], 2)).unwrap();
        assert_eq!(
            updated_json(merge)["hooks"]["Stop"],
            json!([user_group(), app_group("Stop", 2), orca_group()])
        );
        assert_eq!(slots, vec![slot("Stop", 1, app_handler("Stop", 2))]);
    }

    #[test]
    fn fr_hook_002_codex_already_installed_is_unchanged_with_slots() {
        let existing = json!({
            "hooks": { "Stop": [user_group(), app_group("Stop", 1)] }
        })
        .to_string();
        let (merge, slots) = merge_codex_hooks(Some(&existing), &desired(&["Stop"], 1)).unwrap();
        assert_eq!(merge, Merge::Unchanged);
        assert_eq!(slots, vec![slot("Stop", 1, app_handler("Stop", 1))]);
    }

    #[test]
    fn fr_hook_004_codex_invalid_hooks_json_is_rejected() {
        let want = desired(&["Stop"], 1);
        assert_eq!(
            merge_codex_hooks(Some("{"), &want),
            Err(MergeError::InvalidJson)
        );
        assert_eq!(
            merge_codex_hooks(Some("[]"), &want),
            Err(MergeError::NotObject)
        );
    }

    #[test]
    fn fr_hook_002_codex_app_group_of_event_not_desired_is_removed_with_empty_key() {
        let existing = json!({
            "hooks": {
                "PreModelRequest": [app_group("PreModelRequest", 1)],
                "Stop": [app_group("Stop", 1)]
            }
        })
        .to_string();
        let (merge, slots) = merge_codex_hooks(Some(&existing), &desired(&["Stop"], 1)).unwrap();
        assert_eq!(
            updated_json(merge)["hooks"],
            json!({ "Stop": [app_group("Stop", 1)] })
        );
        assert_eq!(slots, vec![slot("Stop", 0, app_handler("Stop", 1))]);
    }

    #[test]
    fn fr_hook_002_codex_event_not_desired_keeps_key_when_other_groups_remain() {
        let existing = json!({
            "hooks": {
                "PreModelRequest": [app_group("PreModelRequest", 1), user_group()],
                "Stop": [app_group("Stop", 1)]
            }
        })
        .to_string();
        let (merge, slots) = merge_codex_hooks(Some(&existing), &desired(&["Stop"], 1)).unwrap();
        let merged = updated_json(merge);
        assert_eq!(merged["hooks"]["PreModelRequest"], json!([user_group()]));
        assert_eq!(merged["hooks"]["Stop"], json!([app_group("Stop", 1)]));
        assert_eq!(slots, vec![slot("Stop", 0, app_handler("Stop", 1))]);
    }

    // codex_hook_hash

    #[test]
    fn fr_hook_006_codex_hash_matches_codex_algorithm() {
        assert_eq!(
            codex_hook_hash("PreToolUse", &echo_hi()),
            ECHO_HI_PRE_TOOL_USE_HASH
        );
        assert_eq!(
            codex_hook_hash(
                "UserPromptSubmit",
                &json!({ "type": "command", "command": "x", "timeout": 5 })
            ),
            X_USER_PROMPT_SUBMIT_HASH
        );
    }

    #[test]
    fn fr_hook_006_codex_hash_ignores_handler_key_order() {
        let reordered: Value =
            serde_json::from_str(r#"{"timeout":600,"command":"echo hi","type":"command"}"#)
                .unwrap();
        assert_eq!(
            codex_hook_hash("PreToolUse", &reordered),
            ECHO_HI_PRE_TOOL_USE_HASH
        );
    }

    #[test]
    fn fr_hook_006_codex_hash_depends_on_event_and_handler() {
        let base = codex_hook_hash("PreToolUse", &echo_hi());
        assert_ne!(codex_hook_hash("PostToolUse", &echo_hi()), base);
        assert_ne!(
            codex_hook_hash(
                "PreToolUse",
                &json!({ "type": "command", "command": "echo hi", "timeout": 5 })
            ),
            base
        );
        assert_ne!(
            codex_hook_hash(
                "PreToolUse",
                &json!({ "type": "command", "command": "echo bye", "timeout": 600 })
            ),
            base
        );
    }

    // merge_codex_config

    #[test]
    fn fr_hook_006_codex_config_created_with_hooks_enabled_and_trust() {
        let slots = [slot("PreToolUse", 0, echo_hi())];
        let (_, doc) = updated_toml(merge_codex_config(None, HOOKS_JSON, &slots));
        assert_eq!(doc["features"]["hooks"].as_bool(), Some(true));
        assert_eq!(
            trusted_hash(&doc, &state_key("pre_tool_use", 0)),
            Some(ECHO_HI_PRE_TOOL_USE_HASH)
        );
    }

    #[test]
    fn fr_hook_006_codex_config_trust_key_uses_group_index() {
        let slots = [
            slot("PreToolUse", 2, echo_hi()),
            slot(
                "UserPromptSubmit",
                1,
                json!({ "type": "command", "command": "x", "timeout": 5 }),
            ),
        ];
        let (_, doc) = updated_toml(merge_codex_config(None, HOOKS_JSON, &slots));
        assert_eq!(
            trusted_hash(&doc, &state_key("pre_tool_use", 2)),
            Some(ECHO_HI_PRE_TOOL_USE_HASH)
        );
        assert_eq!(
            trusted_hash(&doc, &state_key("user_prompt_submit", 1)),
            Some(X_USER_PROMPT_SUBMIT_HASH)
        );
    }

    #[test]
    fn fr_hook_006_codex_hooks_disabled_by_user_is_rejected() {
        let slots = [slot("PreToolUse", 0, echo_hi())];
        assert_eq!(
            merge_codex_config(Some("[features]\nhooks = false\n"), HOOKS_JSON, &slots),
            Err(MergeError::HooksDisabled)
        );
        assert_eq!(
            merge_codex_config(
                Some("[features]\ncodex_hooks = false\n"),
                HOOKS_JSON,
                &slots
            ),
            Err(MergeError::HooksDisabled)
        );
    }

    #[test]
    fn fr_hook_006_codex_legacy_codex_hooks_true_leaves_features_alone() {
        let slots = [slot("PreToolUse", 0, echo_hi())];
        let (_, doc) = updated_toml(merge_codex_config(
            Some("[features]\ncodex_hooks = true\n"),
            HOOKS_JSON,
            &slots,
        ));
        assert_eq!(doc["features"]["codex_hooks"].as_bool(), Some(true));
        assert!(doc["features"].get("hooks").is_none());
        assert_eq!(
            trusted_hash(&doc, &state_key("pre_tool_use", 0)),
            Some(ECHO_HI_PRE_TOOL_USE_HASH)
        );
    }

    #[test]
    fn fr_hook_002_codex_legacy_codex_hooks_true_with_trust_is_unchanged() {
        let key = state_key("pre_tool_use", 0);
        let existing = format!(
            "[features]\ncodex_hooks = true\n\n[hooks.state.\"{key}\"]\ntrusted_hash = \"{ECHO_HI_PRE_TOOL_USE_HASH}\"\n"
        );
        let slots = [slot("PreToolUse", 0, echo_hi())];
        assert_eq!(
            merge_codex_config(Some(&existing), HOOKS_JSON, &slots),
            Ok(Merge::Unchanged)
        );
    }

    #[test]
    fn fr_hook_006_codex_features_without_hooks_keys_gets_hooks_true() {
        let slots = [slot("PreToolUse", 0, echo_hi())];
        let (_, doc) = updated_toml(merge_codex_config(
            Some("[features]\nweb_search = true\n"),
            HOOKS_JSON,
            &slots,
        ));
        assert_eq!(doc["features"]["hooks"].as_bool(), Some(true));
        assert!(doc["features"].get("codex_hooks").is_none());
        assert_eq!(doc["features"]["web_search"].as_bool(), Some(true));
    }

    #[test]
    fn fr_hook_004_codex_config_keeps_other_keys_and_comments() {
        let existing = "# 사용자 주석\nmodel = \"o3\"\n\n[features]\nhooks = true\nweb_search = true\n\n[mcp_servers.docs]\ncommand = \"docs-mcp\"\n";
        let slots = [slot("PreToolUse", 0, echo_hi())];
        let (text, doc) = updated_toml(merge_codex_config(Some(existing), HOOKS_JSON, &slots));
        assert!(text.contains("# 사용자 주석"), "{text}");
        assert_eq!(doc["model"].as_str(), Some("o3"));
        assert_eq!(doc["features"]["hooks"].as_bool(), Some(true));
        assert_eq!(doc["features"]["web_search"].as_bool(), Some(true));
        assert_eq!(
            doc["mcp_servers"]["docs"]["command"].as_str(),
            Some("docs-mcp")
        );
        assert_eq!(
            trusted_hash(&doc, &state_key("pre_tool_use", 0)),
            Some(ECHO_HI_PRE_TOOL_USE_HASH)
        );
    }

    #[test]
    fn fr_hook_006_codex_config_keeps_enabled_and_replaces_stale_hash() {
        let key = state_key("pre_tool_use", 0);
        let existing = format!(
            "[features]\nhooks = true\n\n[hooks.state.\"{key}\"]\nenabled = false\ntrusted_hash = \"sha256:old\"\n"
        );
        let slots = [slot("PreToolUse", 0, echo_hi())];
        let (_, doc) = updated_toml(merge_codex_config(Some(&existing), HOOKS_JSON, &slots));
        assert_eq!(trusted_hash(&doc, &key), Some(ECHO_HI_PRE_TOOL_USE_HASH));
        assert_eq!(
            doc["hooks"]["state"][key.as_str()]["enabled"].as_bool(),
            Some(false)
        );
    }

    #[test]
    fn fr_hook_002_codex_config_already_installed_is_unchanged() {
        let slots = [
            slot("PreToolUse", 0, echo_hi()),
            slot("Stop", 3, app_handler("Stop", 1)),
        ];
        let (first, _) = updated_toml(merge_codex_config(
            Some("model = \"o3\"\n"),
            HOOKS_JSON,
            &slots,
        ));
        assert_eq!(
            merge_codex_config(Some(&first), HOOKS_JSON, &slots),
            Ok(Merge::Unchanged)
        );
    }

    #[test]
    fn fr_hook_004_codex_invalid_config_is_rejected() {
        let slots = [slot("PreToolUse", 0, echo_hi())];
        assert_eq!(
            merge_codex_config(Some("[features\nhooks = "), HOOKS_JSON, &slots),
            Err(MergeError::InvalidToml)
        );
    }

    mod props {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn fr_hook_006_codex_hash_is_sha256_lower_hex(command in "[ -~]{0,40}", timeout in 1u32..1000) {
                let hash = codex_hook_hash(
                    "Stop",
                    &json!({ "type": "command", "command": command, "timeout": timeout }),
                );
                let hex = hash.strip_prefix("sha256:").expect("sha256: 접두사");
                prop_assert_eq!(hex.len(), 64);
                prop_assert!(hex.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
            }

            #[test]
            fn fr_hook_002_codex_slot_index_follows_user_groups(count in 0usize..5) {
                let groups: Vec<Value> = (0..count).map(|_| user_group()).collect();
                let existing = json!({ "hooks": { "Stop": groups } }).to_string();
                let (_, slots) = merge_codex_hooks(Some(&existing), &desired(&["Stop"], 1)).unwrap();
                prop_assert_eq!(slots, vec![slot("Stop", count, app_handler("Stop", 1))]);
            }

            #[test]
            fn fr_hook_002_codex_slot_index_is_existing_app_group_position(count in 0usize..5, at in 0usize..6) {
                let at = at.min(count);
                let mut groups: Vec<Value> = (0..count).map(|_| user_group()).collect();
                groups.insert(at, app_group("Stop", 1));
                let existing = json!({ "hooks": { "Stop": groups } }).to_string();
                let (merge, slots) = merge_codex_hooks(Some(&existing), &desired(&["Stop"], 2)).unwrap();
                let mut expected: Vec<Value> = (0..count).map(|_| user_group()).collect();
                expected.insert(at, app_group("Stop", 2));
                prop_assert_eq!(&updated_json(merge)["hooks"]["Stop"], &Value::Array(expected));
                prop_assert_eq!(slots, vec![slot("Stop", at, app_handler("Stop", 2))]);
            }
        }
    }
}
