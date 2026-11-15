use crate::agent::domain::model::{AgentKind, Environment};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 모델 선택지 하나. `value`는 에이전트에 넘기는 값(`opus`, `gpt-5.6-sol` 등)이다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelOption {
    pub value: String,
    /// 화면에 보이는 이름(`Opus 5.5`, `GPT-5.6-Sol`)
    pub label: String,
    /// 별칭(`default`, `opus` 등)이 지금 가리키는 실제 모델 ID(`claude-opus-5-5`). 별칭이 아니거나 모르면 None.
    /// CLI가 준 설명문은 넣지 않는다.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// 모델 목록을 어디서 얻었는지.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ModelSource {
    /// CLI에서 받았다.
    Queried,
    /// 조회가 실패하거나 시간을 넘겨 고정 목록을 썼다.
    Fallback,
}

/// 모델 목록 조회 결과.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelList {
    pub options: Vec<ModelOption>,
    /// 미리 고를 모델의 `value`. 항상 `options` 안에 있다.
    pub default_value: String,
    pub source: ModelSource,
}

/// 다음을 눌렀을 때 고른 값. 저장 파일과 명령 인자에 그대로 쓴다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Selection {
    pub agent: AgentKind,
    pub environment: Environment,
    pub model: String,
}
