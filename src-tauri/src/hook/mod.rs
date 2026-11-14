//! Claude Code와 Codex에 앱의 훅을 설치하고, 훅이 보낸 이벤트를 로컬 서버로 받는다.
//! 병합, 해시, 요청 해석은 `domain/`, 파일 접근은 `ConfigFs`를 구현한 `platform/`, 서버는 `server.rs`가 맡는다.

pub mod domain;
pub mod fs;
pub mod install;
pub mod server;
