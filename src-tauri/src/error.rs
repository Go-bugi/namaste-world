//! 명령이 돌려주는 에러 타입. renderer는 ts-rs가 만든 같은 이름의 타입으로 받는다.

use serde::Serialize;
use ts_rs::TS;

/// 시작 설정을 마치지 못한 이유.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum OnboardingError {
    /// 작은 창을 만들지 못했다. 시작 설정 창은 닫지 않는다.
    WindowFailed { message: String },
}
