//! 에이전트 CLI를 실행할 환경(셸 환경)의 계산. 셸 출력 해석, 앱 환경과 합치기, 셸 선택, WSL 로그인 셸 스크립트를 다룬다.
//! 셸을 실제로 띄우는 코드는 OS마다 달라 `platform/`에 둔다. 이 모듈에는 IO가 없다.

pub mod choice;
pub mod env;
pub mod model;
pub mod wsl;
