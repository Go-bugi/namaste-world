//! 설치된 에이전트의 로그인 상태를 그 실행 환경의 CLI에 묻는다. 로그인 파일, 키체인, 토큰은 읽지 않는다.
//! 출력 해석은 `domain/`, CLI 실행은 `LoginHost`를 구현한 `platform/`이 맡는다.

pub mod check;
pub mod domain;
pub mod host;
