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
