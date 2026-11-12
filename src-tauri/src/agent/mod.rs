//! 로컬에 설치된 코딩 에이전트(Claude Code, Codex)를 실행 환경별로 감지한다.
//! 판정과 해석은 `domain/`, 환경을 확인하는 IO는 `Host`를 구현한 `platform/`이 맡는다.

pub mod detect;
pub mod domain;
pub mod host;
