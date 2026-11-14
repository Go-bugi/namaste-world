use crate::hook::fs::ConfigFs;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

/// 앱이 도는 OS의 파일 시스템. 경로는 그 OS의 표기 그대로다.
pub struct NativeFs;

impl ConfigFs for NativeFs {
    async fn read(&self, path: &str) -> io::Result<Option<String>> {
        let path = PathBuf::from(path);
        blocking(move || read_file(&path)).await
    }

    async fn write(&self, path: &str, contents: &str, mode: u32) -> io::Result<()> {
        let path = PathBuf::from(path);
        let contents = contents.to_owned();
        blocking(move || write_file(&path, &contents, mode)).await
    }
}

/// 파일 IO를 tokio 작업 스레드 밖에서 돌린다.
pub async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(io::Error::other)?
}

/// 파일 내용. 없으면 `None`이다.
pub fn read_file(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// 파일을 원자적으로 바꾼다.
///
/// - 심볼릭 링크면 링크가 가리키는 파일을 바꾼다. 링크를 일반 파일로 덮어쓰면 dotfiles 저장소 연결이 끊긴다.
/// - 기존 파일은 쓰기 모드로 열어 쓰기 권한을 먼저 확인한다. 권한이 없으면 `PermissionDenied`이고 아무것도 쓰지 않는다.
/// - 같은 폴더의 임시 파일에 쓴 뒤 이름을 바꿔, 중간에 끊겨도 설정 파일이 반쯤 쓰인 채 남지 않는다.
/// - 새 파일은 `mode` 권한으로 만들고, 기존 파일은 원래 권한을 지킨다.
pub fn write_file(path: &Path, contents: &str, mode: u32) -> io::Result<()> {
    let target = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let existing = match fs::metadata(&target) {
        Ok(metadata) => {
            OpenOptions::new().write(true).open(&target)?;
            Some(metadata.permissions())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let dir = target
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "부모 폴더가 없는 경로"))?;
    create_private_dir(dir)?;
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "파일 이름이 없는 경로"))?;
    let temp = dir.join(format!(
        ".{}.{}.namaste-tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    fs::write(&temp, contents)?;
    let permissions = match existing {
        Some(permissions) => permissions,
        None => permissions_for(&temp, mode)?,
    };
    fs::set_permissions(&temp, permissions)
        .and_then(|()| fs::rename(&temp, &target))
        .inspect_err(|_| {
            let _ = fs::remove_file(&temp);
        })
}

/// 폴더가 없으면 만든다. 새로 만든 폴더는 그 사용자만 열 수 있게 한다.
pub fn create_private_dir(dir: &Path) -> io::Result<()> {
    if dir.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dir)?;
    let permissions = permissions_for(dir, 0o700)?;
    fs::set_permissions(dir, permissions)
}

#[cfg(unix)]
fn permissions_for(path: &Path, mode: u32) -> io::Result<fs::Permissions> {
    use std::os::unix::fs::PermissionsExt;
    let _ = path;
    Ok(fs::Permissions::from_mode(mode))
}

/// Windows 파일에는 POSIX 권한이 없어 지금 권한을 그대로 둔다. 사용자 폴더 아래는 기본 ACL이 그 사용자만 쓰게 한다.
#[cfg(not(unix))]
fn permissions_for(path: &Path, _mode: u32) -> io::Result<fs::Permissions> {
    fs::metadata(path).map(|metadata| metadata.permissions())
}
