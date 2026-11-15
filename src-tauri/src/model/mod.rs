//! 에이전트의 모델 목록을 그 실행 환경의 CLI에서 조회하고, 고른 에이전트와 모델을 다룬다.
//! 응답 해석과 목록 결정은 `domain/`, CLI 실행은 `ModelHost`를 구현한 `platform/`이 맡는다.

pub mod domain;
pub mod host;
pub mod query;
