use super::process;
use crate::agent::domain::assemble::{executable_names, search_dirs};
use crate::agent::domain::model::{AgentKind, Found, HostOs};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

/// `--version` 하나에 주는 시간.
const VERSION_LIMIT: Duration = Duration::from_secs(3);

/// 앱이 도는 OS(Windows 자체나 macOS)에서 에이전트 실행 파일을 찾고 버전을 읽는다.
pub async fn probe_native(
    os: HostOs,
    path_env: &str,
    home: &Path,
    app_data: Option<&Path>,
    local_app_data: Option<&Path>,
) -> BTreeMap<AgentKind, Found> {
    let dirs = search_dirs(os, path_env, home, app_data, local_app_data);
    let mut found = BTreeMap::new();
    for agent in AgentKind::ALL {
        if let Some(path) = first_file(&dirs, &executable_names(os, agent)) {
            let version_output = read_version(&path).await;
            found.insert(
                agent,
                Found {
                    path: path.to_string_lossy().into_owned(),
                    version_output,
                },
            );
        }
    }
    found
}

/// 디렉터리 순서대로, 그 안에서는 이름 순서대로 처음 있는 파일.
fn first_file(dirs: &[PathBuf], names: &[String]) -> Option<PathBuf> {
    dirs.iter()
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|path| path.is_file())
}

/// `--version` 출력의 첫 줄. 실패하거나 시간 안에 끝나지 않으면 `None`이다.
async fn read_version(path: &Path) -> Option<String> {
    let mut command = Command::new(path);
    command.arg("--version");
    let output = process::run(command, VERSION_LIMIT).await?;
    process::stdout_text(&output)?
        .lines()
        .next()
        .map(str::to_owned)
}
