use std::future::Future;
use std::io;

/// 한 실행 환경의 파일 접근. `path`는 그 환경 안의 경로 표기다. `platform/`이 환경마다 구현하고 테스트는 메모리 구현을 쓴다.
pub trait ConfigFs: Sync {
    /// 파일 내용. 없으면 `Ok(None)`이다.
    fn read(&self, path: &str) -> impl Future<Output = io::Result<Option<String>>> + Send;

    /// 부모 폴더를 만들고 파일을 바꾼다. 기존 파일에 쓰기 권한이 없으면 `PermissionDenied`다.
    /// `mode`는 POSIX 권한이고 Windows 파일에는 무시한다.
    fn write(
        &self,
        path: &str,
        contents: &str,
        mode: u32,
    ) -> impl Future<Output = io::Result<()>> + Send;
}
