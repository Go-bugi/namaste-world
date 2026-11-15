use std::process::{Output, Stdio};
use std::time::Duration;
use tokio::process::Command;

/// 감지용 외부 프로그램을 실행하고 `limit` 안에 끝나면 출력을 돌려준다.
/// 입력을 기다리지 못하게 stdin을 닫고 stderr는 버린다. 시간을 넘기거나 띄우지 못하면 `None`이고,
/// 이 future가 버려지면(상위 시간 초과 등) 프로세스도 함께 죽인다.
pub async fn run(mut command: Command, limit: Duration) -> Option<Output> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    hide_console(&mut command);
    let child = command.spawn().ok()?;
    tokio::time::timeout(limit, child.wait_with_output())
        .await
        .ok()?
        .ok()
}

/// 성공한 출력의 stdout을 UTF-8 문자열로 돌려준다.
pub fn stdout_text(output: &Output) -> Option<String> {
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Windows에서 콘솔 프로그램을 띄울 때마다 콘솔 창이 깜박이지 않게 한다.
#[cfg(target_os = "windows")]
fn hide_console(command: &mut Command) {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn hide_console(_command: &mut Command) {}

/// 프로그램과 그 자식 프로세스까지 한꺼번에 띄우고 거두는 실행. 모델 목록 조회처럼 npm 래퍼(`node`)가
/// 실제 바이너리를 자식으로 띄우는 CLI에 쓴다. 직접 띄운 프로세스만 죽이면 자식이 남기 때문이다.
/// `run`과 같지만, 시간을 넘기거나 future가 버려지면 프로세스 트리 전체를 죽인다.
pub async fn run_tree(mut command: Command, limit: Duration) -> Option<Output> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    hide_console(&mut command);
    own_group(&mut command);
    let child = command.spawn().ok()?;
    let mut tree = TreeGuard(child.id());
    let output = tokio::time::timeout(limit, child.wait_with_output())
        .await
        .ok()?
        .ok();
    // 정상으로 끝났으면 거둘 프로세스가 없다
    tree.0 = None;
    output
}

/// 외부 프로그램을 띄워 표준 입력에 `input` 한 줄을 쓰고, stdout을 줄 단위로 읽다가 `accept`가 Some을 돌려주면 그 값을 준다.
/// 응답 줄을 받을 때까지 stdin을 열어 둔다. 띄우지 못하거나 응답 전에 출력이 끝나면 None이다.
/// 상한 시간은 부르는 쪽이 건다. 값을 받거나 이 future가 버려지면 프로세스 트리 전체를 죽인다.
pub async fn request_line<T>(
    mut command: Command,
    input: &str,
    accept: impl Fn(&str) -> Option<T>,
) -> Option<T> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    hide_console(&mut command);
    own_group(&mut command);
    let mut child = command.spawn().ok()?;
    // 응답을 받은 뒤에도 프로그램은 다음 입력을 기다리므로 어느 경우든 트리째 거둔다
    let _tree = TreeGuard(child.id());
    let mut stdin = child.stdin.take()?;
    stdin.write_all(input.as_bytes()).await.ok()?;
    stdin.write_all(b"\n").await.ok()?;
    stdin.flush().await.ok()?;
    let mut lines = BufReader::new(child.stdout.take()?).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(value) = accept(&line) {
            return Some(value);
        }
    }
    None
}

/// 버려질 때 프로세스 트리를 죽인다. 값이 None이면 아무것도 하지 않는다.
struct TreeGuard(Option<u32>);

impl Drop for TreeGuard {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            kill_tree(pid);
        }
    }
}

/// 새 프로세스 그룹으로 띄워 그룹째 죽일 수 있게 한다.
#[cfg(unix)]
fn own_group(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
fn own_group(_command: &mut Command) {}

/// 그룹 전체에 SIGKILL을 보낸다. `own_group`으로 띄웠으므로 그룹 ID가 곧 pid다.
#[cfg(unix)]
fn kill_tree(pid: u32) {
    let _ = std::process::Command::new("kill")
        .args(["-KILL", &format!("-{pid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// `cmd.exe`가 띄운 `node.exe`와 그 아래 프로세스까지 `taskkill /T`로 죽인다.
#[cfg(windows)]
fn kill_tree(pid: u32) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = std::process::Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

#[cfg(not(any(unix, windows)))]
fn kill_tree(_pid: u32) {}
