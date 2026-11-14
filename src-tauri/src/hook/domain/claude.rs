use super::hooks_json::merge_hook_groups;
use super::model::{Merge, MergeError};

/// `~/.claude/settings.json`에 앱 훅을 병합한다. `existing`이 `None`이면 빈 객체에서 시작한다.
///
/// `desired`는 (이벤트 이름, handler) 목록이다. 모든 이벤트 배열에서 앱 항목(`HOOK_MARKER`가 든 handler만 가진
/// group)을 뺀 뒤 `desired`의 이벤트마다 `{"hooks":[handler]}` group 하나를 맨 끝에 둔다. 앱 항목을 빼서 비게 된
/// 이벤트가 `desired`에 없으면 키도 지운다. 앱이 아닌 group과 키, 순서는 그대로다.
/// 결과가 지금 내용과 의미상 같으면 `Unchanged`다.
pub fn merge_claude_settings(
    existing: Option<&str>,
    desired: &[(&str, serde_json::Value)],
) -> Result<Merge, MergeError> {
    merge_hook_groups(existing, desired).map(|(merge, _)| merge)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

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

    fn updated(result: Result<Merge, MergeError>) -> Value {
        match result {
            Ok(Merge::Updated(text)) => {
                serde_json::from_str(&text).expect("결과가 JSON이어야 한다")
            }
            other => panic!("Updated가 아니다: {other:?}"),
        }
    }

    fn user_group() -> Value {
        json!({ "hooks": [{ "type": "command", "command": "say done" }] })
    }

    fn orca_group() -> Value {
        json!({ "matcher": "", "hooks": [{ "type": "command", "command": "orca-hook stop", "timeout": 10 }] })
    }

    #[test]
    fn fr_hook_002_first_install_registers_each_event_once() {
        let want = desired(&["SessionStart", "PreToolUse", "Stop"], 1);
        let merged = updated(merge_claude_settings(None, &want));
        for (event, handler) in &want {
            assert_eq!(
                merged["hooks"][event],
                json!([{ "hooks": [handler] }]),
                "{event}"
            );
        }
    }

    #[test]
    fn fr_hook_006_missing_file_starts_from_empty_object() {
        let want = desired(&["Stop"], 1);
        let merged = updated(merge_claude_settings(None, &want));
        assert_eq!(
            merged,
            json!({ "hooks": { "Stop": [app_group("Stop", 1)] } })
        );
    }

    #[test]
    fn fr_hook_002_already_installed_is_unchanged() {
        let want = desired(&["SessionStart", "PreToolUse", "Stop"], 1);
        let first = match merge_claude_settings(None, &want) {
            Ok(Merge::Updated(text)) => text,
            other => panic!("Updated가 아니다: {other:?}"),
        };
        assert_eq!(
            merge_claude_settings(Some(&first), &want),
            Ok(Merge::Unchanged)
        );
    }

    #[test]
    fn fr_hook_002_semantically_equal_file_is_unchanged() {
        let existing = json!({
            "model": "opus",
            "hooks": { "Stop": [user_group(), app_group("Stop", 1)] }
        });
        let pretty = serde_json::to_string_pretty(&existing).unwrap();
        let compact = serde_json::to_string(&existing).unwrap();
        let want = desired(&["Stop"], 1);
        assert_eq!(
            merge_claude_settings(Some(&pretty), &want),
            Ok(Merge::Unchanged)
        );
        assert_eq!(
            merge_claude_settings(Some(&compact), &want),
            Ok(Merge::Unchanged)
        );
    }

    #[test]
    fn fr_hook_002_stale_app_group_is_replaced_in_place_without_duplicates() {
        let existing = json!({
            "hooks": { "Stop": [app_group("Stop", 1), user_group()] }
        })
        .to_string();
        let merged = updated(merge_claude_settings(
            Some(&existing),
            &desired(&["Stop"], 2),
        ));
        assert_eq!(
            merged["hooks"]["Stop"],
            json!([app_group("Stop", 2), user_group()])
        );
    }

    #[test]
    fn fr_hook_002_duplicate_app_groups_collapse_into_first_position() {
        let existing = json!({
            "hooks": {
                "Stop": [user_group(), app_group("Stop", 1), orca_group(), app_group("Stop", 1)]
            }
        })
        .to_string();
        let merged = updated(merge_claude_settings(
            Some(&existing),
            &desired(&["Stop"], 2),
        ));
        assert_eq!(
            merged["hooks"]["Stop"],
            json!([user_group(), app_group("Stop", 2), orca_group()])
        );
    }

    #[test]
    fn fr_hook_004_app_group_before_other_tool_is_not_moved() {
        let existing = json!({
            "hooks": { "Stop": [app_group("Stop", 1), orca_group()] }
        })
        .to_string();
        assert_eq!(
            merge_claude_settings(Some(&existing), &desired(&["Stop"], 1)),
            Ok(Merge::Unchanged)
        );
        let merged = updated(merge_claude_settings(
            Some(&existing),
            &desired(&["Stop"], 2),
        ));
        assert_eq!(
            merged["hooks"]["Stop"],
            json!([app_group("Stop", 2), orca_group()])
        );
    }

    #[test]
    fn fr_hook_004_app_group_after_other_tool_stays_after_it() {
        let existing = json!({
            "hooks": { "Stop": [orca_group(), app_group("Stop", 1)] }
        })
        .to_string();
        let merged = updated(merge_claude_settings(
            Some(&existing),
            &desired(&["Stop"], 2),
        ));
        assert_eq!(
            merged["hooks"]["Stop"],
            json!([orca_group(), app_group("Stop", 2)])
        );
    }

    #[test]
    fn fr_hook_002_stale_app_group_with_marker_in_args_is_replaced() {
        let old = json!({
            "hooks": [{
                "type": "command",
                "command": "cmd.exe",
                "args": ["/d", "/c", "old namaste-world-hook"],
                "timeout": 5
            }]
        });
        let existing = json!({ "hooks": { "Stop": [old, user_group()] } }).to_string();
        let merged = updated(merge_claude_settings(
            Some(&existing),
            &desired(&["Stop"], 2),
        ));
        assert_eq!(
            merged["hooks"]["Stop"],
            json!([app_group("Stop", 2), user_group()])
        );
    }

    #[test]
    fn fr_hook_004_other_tools_hooks_keep_content_and_order() {
        let existing = json!({
            "hooks": { "Stop": [user_group(), orca_group()] }
        })
        .to_string();
        let merged = updated(merge_claude_settings(
            Some(&existing),
            &desired(&["Stop"], 1),
        ));
        assert_eq!(
            merged["hooks"]["Stop"],
            json!([user_group(), orca_group(), app_group("Stop", 1)])
        );
    }

    #[test]
    fn fr_hook_004_group_mixing_app_and_user_handlers_is_kept() {
        let mixed = json!({
            "hooks": [
                { "type": "command", "command": "say done" },
                app_handler("Stop", 1)
            ]
        });
        let existing = json!({ "hooks": { "Stop": [mixed.clone()] } }).to_string();
        let merged = updated(merge_claude_settings(
            Some(&existing),
            &desired(&["Stop"], 2),
        ));
        assert_eq!(
            merged["hooks"]["Stop"],
            json!([mixed, app_group("Stop", 2)])
        );
    }

    #[test]
    fn fr_hook_004_non_hook_settings_and_other_events_are_kept() {
        let existing = json!({
            "permissions": { "allow": ["Bash(ls)"], "deny": [] },
            "model": "opus",
            "hooks": {
                "Notification": [user_group()],
                "Stop": [orca_group()]
            },
            "env": { "FOO": "1" }
        });
        let merged = updated(merge_claude_settings(
            Some(&existing.to_string()),
            &desired(&["Stop"], 1),
        ));
        assert_eq!(merged["permissions"], existing["permissions"]);
        assert_eq!(merged["model"], existing["model"]);
        assert_eq!(merged["env"], existing["env"]);
        assert_eq!(merged["hooks"]["Notification"], json!([user_group()]));
        let keys: Vec<&String> = merged.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["permissions", "model", "hooks", "env"]);
    }

    #[test]
    fn fr_hook_004_invalid_json_is_rejected() {
        let want = desired(&["Stop"], 1);
        assert_eq!(
            merge_claude_settings(Some("{ \"model\": "), &want),
            Err(MergeError::InvalidJson)
        );
        assert_eq!(
            merge_claude_settings(Some("not json"), &want),
            Err(MergeError::InvalidJson)
        );
    }

    #[test]
    fn fr_hook_004_unexpected_shape_is_rejected() {
        let want = desired(&["Stop"], 1);
        assert_eq!(
            merge_claude_settings(Some("[]"), &want),
            Err(MergeError::NotObject)
        );
        assert_eq!(
            merge_claude_settings(Some(r#"{"hooks": "none"}"#), &want),
            Err(MergeError::NotObject)
        );
    }

    #[test]
    fn fr_hook_002_app_group_of_event_not_desired_is_removed_with_empty_key() {
        let existing = json!({
            "hooks": {
                "PreModelRequest": [app_group("PreModelRequest", 1)],
                "Stop": [app_group("Stop", 1)]
            }
        })
        .to_string();
        let merged = updated(merge_claude_settings(
            Some(&existing),
            &desired(&["Stop"], 1),
        ));
        assert_eq!(merged["hooks"], json!({ "Stop": [app_group("Stop", 1)] }));
    }

    #[test]
    fn fr_hook_002_event_not_desired_keeps_key_when_other_groups_remain() {
        let existing = json!({
            "hooks": {
                "PreModelRequest": [user_group(), app_group("PreModelRequest", 1), orca_group()],
                "Stop": [app_group("Stop", 1)]
            }
        })
        .to_string();
        let merged = updated(merge_claude_settings(
            Some(&existing),
            &desired(&["Stop"], 1),
        ));
        assert_eq!(
            merged["hooks"]["PreModelRequest"],
            json!([user_group(), orca_group()])
        );
        assert_eq!(merged["hooks"]["Stop"], json!([app_group("Stop", 1)]));
    }

    mod props {
        use super::*;
        use proptest::prelude::*;

        fn user_groups() -> impl Strategy<Value = Vec<Value>> {
            prop::collection::vec("[a-z ]{1,12}", 0..5).prop_map(|commands| {
                commands
                    .into_iter()
                    .map(|c| json!({ "hooks": [{ "type": "command", "command": c }] }))
                    .collect()
            })
        }

        proptest! {
            #[test]
            fn fr_hook_004_user_groups_survive_in_order(groups in user_groups(), version in 1u32..4) {
                let existing = json!({ "hooks": { "Stop": groups.clone() } }).to_string();
                let merged = updated(merge_claude_settings(Some(&existing), &desired(&["Stop"], version)));
                let mut expected = groups;
                expected.push(app_group("Stop", version));
                prop_assert_eq!(&merged["hooks"]["Stop"], &Value::Array(expected));
            }

            #[test]
            fn fr_hook_004_existing_app_group_keeps_its_position(groups in user_groups(), at in 0usize..6, old in 1u32..4, new in 4u32..6) {
                let at = at.min(groups.len());
                let mut stop = groups.clone();
                stop.insert(at, app_group("Stop", old));
                let existing = json!({ "hooks": { "Stop": stop } }).to_string();
                let merged = updated(merge_claude_settings(Some(&existing), &desired(&["Stop"], new)));
                let mut expected = groups;
                expected.insert(at, app_group("Stop", new));
                prop_assert_eq!(&merged["hooks"]["Stop"], &Value::Array(expected));
            }

            #[test]
            fn fr_hook_002_second_merge_is_unchanged(groups in user_groups(), old in 1u32..4, new in 4u32..6) {
                let mut stop = groups;
                stop.insert(0, app_group("Stop", old));
                let existing = json!({ "hooks": { "Stop": stop } }).to_string();
                let want = desired(&["Stop", "PreToolUse"], new);
                let first = match merge_claude_settings(Some(&existing), &want) {
                    Ok(Merge::Updated(text)) => text,
                    other => panic!("Updated가 아니다: {other:?}"),
                };
                prop_assert_eq!(merge_claude_settings(Some(&first), &want), Ok(Merge::Unchanged));
            }
        }
    }
}
