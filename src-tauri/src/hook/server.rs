use super::domain::model::HookEvent;
use super::domain::request::{parse_full, parse_summary, route, Rejection, FULL_BODY_LIMIT};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;
use futures_util::{StreamExt, TryStreamExt};
use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio_util::io::{StreamReader, SyncIoBridge};

/// 훅 요청의 토큰 헤더.
const TOKEN_HEADER: &str = "x-namaste-token";

/// 127.0.0.1의 빈 포트에 묶는다. 다른 기기에서는 접속할 수 없다.
pub async fn bind() -> io::Result<(TcpListener, u16)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let port = listener.local_addr()?.port();
    Ok((listener, port))
}

/// `POST /hook/<agent>/<Event>`를 받아 `on_event`로 넘긴다. 토큰이 틀리면 401, 경로가 틀리면 404,
/// 그 밖에는 본문을 해석하지 못해도 204다. 훅은 응답을 보지 않으므로 해석 실패를 알리지 않는다.
pub async fn serve(
    listener: TcpListener,
    token: String,
    on_event: impl Fn(HookEvent) + Send + Sync + 'static,
) {
    let shared = Arc::new(Shared {
        token,
        on_event: Box::new(on_event),
    });
    let app = Router::new()
        .route("/hook/{agent}/{event}", post(receive))
        .fallback(|| async { StatusCode::NOT_FOUND })
        .with_state(shared);
    if let Err(error) = axum::serve(listener, app).await {
        eprintln!("훅 서버가 멈췄습니다: {error}");
    }
}

struct Shared {
    token: String,
    on_event: Box<dyn Fn(HookEvent) + Send + Sync>,
}

async fn receive(
    State(shared): State<Arc<Shared>>,
    Path((agent, event)): Path<(String, String)>,
    headers: HeaderMap,
    body: Body,
) -> StatusCode {
    let given = headers
        .get(TOKEN_HEADER)
        .and_then(|value| value.to_str().ok());
    let agent = match route(&agent, &shared.token, given) {
        Ok(agent) => agent,
        Err(Rejection::Unauthorized) => return StatusCode::UNAUTHORIZED,
        Err(Rejection::NotFound) => return StatusCode::NOT_FOUND,
    };
    let received_at = now_ms();
    if let Some(hook_event) = read_event(agent, event, body, received_at).await {
        (shared.on_event)(hook_event);
    }
    StatusCode::NO_CONTENT
}

/// 본문을 `FULL_BODY_LIMIT`까지 메모리에 읽는다. 그 안에 끝나면 전체를 해석하고, 넘으면 이미 읽은 부분과
/// 남은 스트림을 이어 요약 필드만 스트림으로 해석한다. 그래서 본문 크기와 상관없이 메모리는 1MB 근처로 묶인다.
async fn read_event(
    agent: crate::agent::domain::model::AgentKind,
    event: String,
    body: Body,
    received_at: u64,
) -> Option<HookEvent> {
    let mut stream = body.into_data_stream();
    let mut head = Vec::new();
    while head.len() <= FULL_BODY_LIMIT {
        match stream.next().await {
            Some(Ok(chunk)) => head.extend_from_slice(&chunk),
            Some(Err(_)) => return None,
            None => return parse_full(agent, &event, &head, received_at),
        }
    }
    let rest = StreamReader::new(stream.map_err(io::Error::other));
    let reader = SyncIoBridge::new(std::io::Cursor::new(head).chain(rest));
    tokio::task::spawn_blocking(move || parse_summary(agent, &event, reader, received_at))
        .await
        .ok()
        .flatten()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::domain::model::AgentKind;
    use serde_json::{json, Value};
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    const TOKEN: &str = "test-token-0123456789abcdef";
    const WAIT: Duration = Duration::from_secs(10);

    /// 서버를 띄우고 포트와 받은 이벤트 수신기를 돌려준다.
    async fn start() -> (u16, mpsc::UnboundedReceiver<HookEvent>) {
        let (listener, port) = bind().await.expect("bind");
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(serve(listener, TOKEN.to_string(), move |event| {
            let _ = tx.send(event);
        }));
        (port, rx)
    }

    /// 원시 HTTP/1.1 요청을 보내고 응답 상태 코드를 돌려준다.
    async fn post(port: u16, path: &str, token: Option<&str>, body: &[u8]) -> u16 {
        timeout(WAIT, async {
            let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
                .await
                .expect("connect");
            let token_header = token
                .map(|t| format!("X-Namaste-Token: {t}\r\n"))
                .unwrap_or_default();
            let head = format!(
                "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{token_header}\
                 Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).await.expect("write head");
            // 서버가 본문을 다 읽지 않고 응답할 수 있으므로 본문 쓰기 실패는 무시한다
            let _ = stream.write_all(body).await;
            let mut response = Vec::new();
            let _ = stream.read_to_end(&mut response).await;
            let text = String::from_utf8_lossy(&response);
            let status_line = text.lines().next().unwrap_or_default().to_string();
            status_line
                .split_whitespace()
                .nth(1)
                .and_then(|code| code.parse().ok())
                .unwrap_or_else(|| panic!("no status line: {status_line:?}"))
        })
        .await
        .expect("response in time")
    }

    async fn next_event(rx: &mut mpsc::UnboundedReceiver<HookEvent>) -> HookEvent {
        timeout(WAIT, rx.recv())
            .await
            .expect("event in time")
            .expect("channel open")
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis() as u64
    }

    fn body_of(value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).expect("json")
    }

    /// 거부된 요청 뒤에 올바른 요청을 보내, 다음으로 받는 이벤트가 올바른 요청의 것인지 확인한다.
    async fn assert_next_is_marker(port: u16, rx: &mut mpsc::UnboundedReceiver<HookEvent>) {
        let marker = json!({"hook_event_name": "Stop", "session_id": "marker"});
        assert_eq!(
            post(port, "/hook/claude/Stop", Some(TOKEN), &body_of(&marker)).await,
            204
        );
        let got = next_event(rx).await;
        assert_eq!(got.session_id.as_deref(), Some("marker"));
    }

    #[tokio::test]
    async fn fr_hook_001_binds_only_to_loopback() {
        let (listener, port) = bind().await.expect("bind");
        let addr = listener.local_addr().expect("local addr");

        assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(addr.port(), port);
        assert_ne!(port, 0);
    }

    #[tokio::test]
    async fn fr_hook_001_two_binds_get_different_free_ports() {
        let (_a, port_a) = bind().await.expect("bind a");
        let (_b, port_b) = bind().await.expect("bind b");

        assert_ne!(port_a, port_b);
    }

    #[tokio::test]
    async fn fr_hook_001_and_009_request_with_token_is_passed_on_as_event() {
        let (port, mut rx) = start().await;
        let input = json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "s-1",
            "cwd": "/Users/tester/project",
            "prompt": "hello"
        });
        let before = now_ms();

        let status = post(
            port,
            "/hook/claude/UserPromptSubmit",
            Some(TOKEN),
            &body_of(&input),
        )
        .await;
        let got = next_event(&mut rx).await;
        let after = now_ms();

        assert_eq!(status, 204);
        assert!(
            (before..=after).contains(&got.received_at),
            "received_at {} not in {before}..={after}",
            got.received_at
        );
        assert_eq!(
            got,
            HookEvent {
                agent: AgentKind::Claude,
                event: "UserPromptSubmit".to_string(),
                received_at: got.received_at,
                session_id: Some("s-1".to_string()),
                tool_name: None,
                tool_use_id: None,
                is_error: None,
                cwd: Some("/Users/tester/project".to_string()),
                payload: Some(input),
                truncated: false,
            }
        );
    }

    #[tokio::test]
    async fn fr_hook_009_codex_event_takes_agent_from_path_and_name_from_url_when_body_has_none() {
        let (port, mut rx) = start().await;
        let input = json!({"session_id": "c-1"});

        let status = post(port, "/hook/codex/Stop", Some(TOKEN), &body_of(&input)).await;
        let got = next_event(&mut rx).await;

        assert_eq!(status, 204);
        assert_eq!(got.agent, AgentKind::Codex);
        assert_eq!(got.event, "Stop");
        assert_eq!(got.session_id.as_deref(), Some("c-1"));
        assert_eq!(got.payload, Some(input));
    }

    #[tokio::test]
    async fn fr_hook_001_request_without_token_is_rejected_and_not_passed_on() {
        let (port, mut rx) = start().await;
        let input = json!({"hook_event_name": "Stop", "session_id": "intruder"});

        let status = post(port, "/hook/claude/Stop", None, &body_of(&input)).await;

        assert_eq!(status, 401);
        assert_next_is_marker(port, &mut rx).await;
    }

    #[tokio::test]
    async fn fr_hook_001_request_with_other_token_is_rejected_and_not_passed_on() {
        let (port, mut rx) = start().await;
        let input = json!({"hook_event_name": "Stop", "session_id": "intruder"});

        let wrong = post(
            port,
            "/hook/claude/Stop",
            Some("test-token-0123456789abcdeX"),
            &body_of(&input),
        )
        .await;
        let short = post(port, "/hook/claude/Stop", Some("x"), &body_of(&input)).await;
        let empty = post(port, "/hook/claude/Stop", Some(""), &body_of(&input)).await;

        assert_eq!((wrong, short, empty), (401, 401, 401));
        assert_next_is_marker(port, &mut rx).await;
    }

    #[tokio::test]
    async fn fr_hook_001_unknown_agent_path_is_404_and_not_passed_on() {
        let (port, mut rx) = start().await;
        let input = json!({"hook_event_name": "Stop", "session_id": "intruder"});

        let status = post(port, "/hook/gemini/Stop", Some(TOKEN), &body_of(&input)).await;
        let other = post(port, "/other", Some(TOKEN), &body_of(&input)).await;

        assert_eq!((status, other), (404, 404));
        assert_next_is_marker(port, &mut rx).await;
    }

    #[tokio::test]
    async fn fr_hook_009_non_json_body_is_ignored_and_server_keeps_running() {
        let (port, mut rx) = start().await;

        let status = post(port, "/hook/claude/Stop", Some(TOKEN), b"not json {").await;
        let array = post(port, "/hook/claude/Stop", Some(TOKEN), b"[1,2,3]").await;

        assert_eq!((status, array), (204, 204));
        assert_next_is_marker(port, &mut rx).await;
    }

    #[tokio::test]
    async fn fr_hook_013_body_over_1mb_passes_only_summary_fields() {
        let (port, mut rx) = start().await;
        // 도구 출력 때문에 3MB가 된 PostToolUse 본문
        let input = json!({
            "hook_event_name": "PostToolUse",
            "session_id": "s-big",
            "tool_name": "Bash",
            "tool_use_id": "toolu_01",
            "cwd": "/Users/tester/project",
            "tool_response": {"stdout": "x".repeat(3 * 1024 * 1024)}
        });
        let body = body_of(&input);
        assert!(body.len() > 3 * 1024 * 1024);

        let status = post(port, "/hook/claude/PostToolUse", Some(TOKEN), &body).await;
        let got = next_event(&mut rx).await;

        assert_eq!(status, 204);
        assert_eq!(
            got,
            HookEvent {
                agent: AgentKind::Claude,
                event: "PostToolUse".to_string(),
                received_at: got.received_at,
                session_id: Some("s-big".to_string()),
                tool_name: Some("Bash".to_string()),
                tool_use_id: Some("toolu_01".to_string()),
                is_error: None,
                cwd: Some("/Users/tester/project".to_string()),
                payload: None,
                truncated: true,
            }
        );
    }

    #[tokio::test]
    async fn fr_hook_013_body_of_200kb_passes_whole_input() {
        let (port, mut rx) = start().await;
        let input = json!({
            "hook_event_name": "PostToolUse",
            "session_id": "s-mid",
            "tool_name": "Read",
            "tool_use_id": "toolu_02",
            "tool_response": {"stdout": "y".repeat(200 * 1024)}
        });
        let body = body_of(&input);
        assert!(body.len() > 200 * 1024 && body.len() <= 1024 * 1024);

        let status = post(port, "/hook/claude/PostToolUse", Some(TOKEN), &body).await;
        let got = next_event(&mut rx).await;

        assert_eq!(status, 204);
        assert_eq!(got.payload, Some(input));
        assert!(!got.truncated);
        assert_eq!(got.tool_name.as_deref(), Some("Read"));
    }

    #[tokio::test]
    async fn fr_hook_010_events_are_passed_on_in_received_order() {
        let (port, mut rx) = start().await;
        let names = ["UserPromptSubmit", "PreToolUse", "PostToolUse", "Stop"];

        for name in names {
            let input = json!({"hook_event_name": name, "session_id": "s-order"});
            let path = format!("/hook/claude/{name}");
            assert_eq!(post(port, &path, Some(TOKEN), &body_of(&input)).await, 204);
        }
        let mut got = Vec::new();
        for _ in names {
            got.push(next_event(&mut rx).await.event);
        }

        assert_eq!(got, names);
    }
}
