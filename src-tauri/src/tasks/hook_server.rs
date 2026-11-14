use crate::events::HOOK_EVENT;
use crate::hook::domain::endpoint::endpoint_file;
use crate::hook::server;
use crate::platform;
use crate::tasks::hook_install;
use tauri::Emitter;

/// 훅 서버를 띄우고 엔드포인트 파일을 쓴 뒤, 앱 시작 감지를 기다려 훅을 설치한다.
/// 엔드포인트 파일을 먼저 써 두어 설치 직후의 훅도 이 서버에 닿게 한다.
/// 서버를 열지 못해도 설치는 하고 결과를 보관해, 화면이 설치 결과를 기다리며 멈추지 않게 한다.
pub fn spawn(app: tauri::AppHandle) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let server = start().await;
        hook_install::spawn_after_startup_detection(app.clone());
        let Some((listener, token)) = server else {
            return;
        };
        // 이벤트 내용은 디스크에 남기지 않는다. renderer로만 보낸다
        server::serve(listener, token, move |event| {
            let _ = app.emit(HOOK_EVENT, event);
        })
        .await;
    })
}

/// 토큰을 만들고 포트를 열고 엔드포인트 파일을 쓴다. 서버를 열지 못하면 `None`이다.
async fn start() -> Option<(tokio::net::TcpListener, String)> {
    let Some(token) = new_token() else {
        eprintln!("훅 서버 토큰을 만들지 못했습니다");
        return None;
    };
    let (listener, port) = match server::bind().await {
        Ok(bound) => bound,
        Err(error) => {
            eprintln!("훅 서버를 열지 못했습니다: {error}");
            return None;
        }
    };
    if let Err(error) = write_endpoint(port, &token).await {
        eprintln!("훅 엔드포인트 파일을 쓰지 못했습니다: {error}");
    }
    Some((listener, token))
}

/// 앱이 끝날 때 엔드포인트 파일을 지운다. 남겨 두면 앱이 꺼진 뒤 그 포트를 차지한 다른 프로세스로
/// 훅 본문이 갈 수 있다.
pub fn remove_endpoint() {
    if let Some(dir) = platform::app_dir() {
        let _ = std::fs::remove_file(dir.join("endpoint"));
    }
}

/// 앱을 실행할 때마다 새로 만드는 토큰(32바이트, 소문자 hex).
fn new_token() -> Option<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// `~/.namaste-world/endpoint`에 포트와 토큰을 쓴다. 그 사용자만 읽을 수 있다.
async fn write_endpoint(port: u16, token: &str) -> std::io::Result<()> {
    let dir =
        platform::app_dir().ok_or_else(|| std::io::Error::other("홈 폴더를 찾지 못했습니다"))?;
    let contents = endpoint_file(port, token);
    tauri::async_runtime::spawn_blocking(move || {
        platform::write_file(&dir.join("endpoint"), &contents, 0o600)
    })
    .await
    .map_err(std::io::Error::other)?
}
