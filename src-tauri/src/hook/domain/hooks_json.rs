use super::command::HOOK_MARKER;
use super::model::{Merge, MergeError};
use serde_json::{json, Map, Value};

/// 앱 group이 놓인 곳. (이벤트 이름, 그 이벤트 배열 안의 위치, handler)
pub(super) type Placed = (String, usize, Value);

/// `{"hooks": {Event: [group]}}` 모양 설정 파일에 앱 훅을 병합한다. Claude Code `settings.json`과 Codex `hooks.json`이
/// 같은 모양이라 둘이 함께 쓴다.
///
/// `desired`의 이벤트마다 앱 group이 있으면 첫 앱 group 자리에서 `{"hooks":[handler]}`로 바꾸고 나머지 앱 group은
/// 지운다. 없으면 맨 끝에 붙인다. 앱 group을 옮기지 않으므로 다른 group의 위치도 바뀌지 않는다. Codex 신뢰 키가
/// group 위치로 정해져, 위치가 바뀌면 다른 도구의 신뢰가 풀리기 때문이다.
/// `desired`에 없는 이벤트의 앱 group은 지우고, 그래서 비게 된 이벤트는 키도 지운다.
pub(super) fn merge_hook_groups(
    existing: Option<&str>,
    desired: &[(&str, Value)],
) -> Result<(Merge, Vec<Placed>), MergeError> {
    let original: Option<Value> = existing
        .map(|text| serde_json::from_str(text).map_err(|_| MergeError::InvalidJson))
        .transpose()?;
    let mut root = match &original {
        Some(Value::Object(map)) => map.clone(),
        Some(_) => return Err(MergeError::NotObject),
        None => Map::new(),
    };
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or(MergeError::NotObject)?;

    let mut emptied = Vec::new();
    for (event, groups) in hooks.iter_mut() {
        let groups = groups.as_array_mut().ok_or(MergeError::NotObject)?;
        if desired.iter().any(|(wanted, _)| wanted == event) {
            continue;
        }
        let before = groups.len();
        groups.retain(|group| !is_app_group(group));
        if groups.len() != before && groups.is_empty() {
            emptied.push(event.clone());
        }
    }
    hooks.retain(|event, _| !emptied.contains(event));

    let placed = desired
        .iter()
        .map(|(event, handler)| {
            let groups = hooks
                .entry(*event)
                .or_insert_with(|| Value::Array(Vec::new()))
                .as_array_mut()
                .ok_or(MergeError::NotObject)?;
            let index = place_app_group(groups, json!({ "hooks": [handler] }));
            Ok(((*event).to_owned(), index, handler.clone()))
        })
        .collect::<Result<Vec<_>, MergeError>>()?;

    let merged = Value::Object(root);
    let merge = if original.as_ref() == Some(&merged) {
        Merge::Unchanged
    } else {
        // to_string_pretty는 Value에 대해 실패하지 않는다
        let text = serde_json::to_string_pretty(&merged).unwrap_or_default();
        Merge::Updated(text + "\n")
    };
    Ok((merge, placed))
}

/// 앱 group을 첫 앱 group 자리에 두고 나머지 앱 group은 지운다. 앱 group이 없으면 끝에 붙인다. 놓인 위치를 돌려준다.
fn place_app_group(groups: &mut Vec<Value>, group: Value) -> usize {
    match groups.iter().position(is_app_group) {
        Some(first) => {
            let mut index = 0;
            groups.retain(|existing| {
                let keep = index == first || !is_app_group(existing);
                index += 1;
                keep
            });
            groups[first] = group;
            first
        }
        None => {
            groups.push(group);
            groups.len() - 1
        }
    }
}

/// handler가 모두 앱 handler인 group. 사용자 handler와 섞인 group은 사용자 것으로 보고 건드리지 않는다.
fn is_app_group(group: &Value) -> bool {
    group
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|handlers| {
            !handlers.is_empty()
                && handlers
                    .iter()
                    .all(|handler| handler.to_string().contains(HOOK_MARKER))
        })
}
