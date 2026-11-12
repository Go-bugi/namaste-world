use super::domain::model::{EnvProbe, Environment, Target};
use std::future::Future;

/// 확인할 환경 목록과 환경별 프로브. `platform/`이 OS마다 구현하고, 테스트는 가짜 구현을 쓴다.
pub trait Host: Sync {
    /// 확인할 실행 환경. macOS는 `[Macos]`, Windows는 `[Windows, Wsl{..}...]`이다.
    /// WSL 배포판의 `running`은 `wsl.exe -l --running -q`에 있는지로 정한다. 목록을 읽지 못하면 WSL 없이 `[Windows]`다.
    fn targets(&self) -> impl Future<Output = Vec<Target>> + Send;

    /// 한 환경에서 에이전트 실행 파일을 찾고 `--version`을 읽는다.
    fn probe(&self, environment: &Environment) -> impl Future<Output = EnvProbe> + Send;
}
