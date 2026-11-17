use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use ts_rs::TS;

/// Windows에서 CLI 실행 환경을 읽을 셸(FR-START-011).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ShellKind {
    PowerShell,
    Cmd,
    GitBash,
}

/// Windows 셸 선택 단계에 보여 줄 것. macOS에서는 명령이 None을 돌려준다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ShellSetup {
    pub options: Vec<ShellKind>,
    pub selected: ShellKind,
}

/// 셸 환경을 읽지 못한 이유(FR-AGENT-018). 문자열을 담지 않아 환경 값이 섞일 수 없다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ShellEnvFailure {
    /// 시간 상한 안에 끝나지 않았다.
    Timeout,
    /// 셸을 띄우지 못했다.
    LaunchFailed,
    /// 셸이 오류 코드로 끝났다.
    ExitedWithError,
    /// 출력에서 환경을 읽지 못했다.
    UnreadableOutput,
}

/// CLI를 실행할 환경. 셸에서 읽은 값과 앱이 받은 값을 합친 것이다.
/// API 키 같은 값이 들어 있으므로 `Serialize`를 붙이지 않고 `Debug`는 값을 찍지 않는다(FR-AGENT-017).
#[derive(Clone, PartialEq, Eq, Default)]
pub struct ShellEnv {
    vars: BTreeMap<String, String>,
}

impl ShellEnv {
    pub fn new(vars: BTreeMap<String, String>) -> Self {
        Self { vars }
    }

    /// 모든 변수. `Command::envs`에 넘긴다.
    pub fn vars(&self) -> impl Iterator<Item = (&str, &str)> {
        self.vars.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// 이름이 정확히 같은 변수의 값.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }
}

/// 키 개수만 찍는다. 이름과 값은 찍지 않는다.
impl fmt::Debug for ShellEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ShellEnv {{ {} vars }}", self.vars.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr_agent_017_debug_does_not_show_names_or_values() {
        let env = ShellEnv::new(BTreeMap::from([
            (
                "ANTHROPIC_API_KEY".to_string(),
                "sk-ant-secret-value".to_string(),
            ),
            (
                "CLAUDE_CONFIG_DIR".to_string(),
                "/Users/a/work-claude".to_string(),
            ),
        ]));

        let printed = format!("{env:?} {env:#?}");

        assert!(!printed.contains("sk-ant-secret-value"), "{printed}");
        assert!(!printed.contains("ANTHROPIC_API_KEY"), "{printed}");
        assert!(!printed.contains("/Users/a/work-claude"), "{printed}");
    }

    #[test]
    fn fr_agent_018_failure_reasons_serialize_without_any_text_payload() {
        let all = [
            (ShellEnvFailure::Timeout, "\"timeout\""),
            (ShellEnvFailure::LaunchFailed, "\"launchFailed\""),
            (ShellEnvFailure::ExitedWithError, "\"exitedWithError\""),
            (ShellEnvFailure::UnreadableOutput, "\"unreadableOutput\""),
        ];

        for (failure, json) in all {
            assert_eq!(serde_json::to_string(&failure).unwrap(), json);
        }
    }
}
