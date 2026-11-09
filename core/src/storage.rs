use crate::{Error, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const DEFAULT_BACKUPS: usize = 5;

#[derive(Debug)]
pub struct FileLock {
    file: File,
    pub lock_path: PathBuf,
}

impl FileLock {
    pub fn release(self) {
        let _ = self.file.unlock();
    }
}

pub fn lock_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".lock");
    PathBuf::from(s)
}

pub fn backup_path(path: &Path, n: usize) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(format!(".bak{n}"));
    PathBuf::from(s)
}

pub fn tmp_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".tmp");
    PathBuf::from(s)
}

pub fn acquire_lock(path: &Path) -> Result<FileLock> {
    let lp = lock_path(path);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lp)?;
    set_private(&lp)?;
    match file.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Err(Error::Locked),
        Err(std::fs::TryLockError::Error(e)) => return Err(Error::Io(e)),
    }
    Ok(FileLock {
        file,
        lock_path: lp,
    })
}

pub fn save_atomic(path: &Path, data: &[u8], backups: usize) -> Result<()> {
    if backups > 0 {
        for i in (1..=backups).rev() {
            let from = if i == 1 {
                path.to_path_buf()
            } else {
                backup_path(path, i - 1)
            };
            let to = backup_path(path, i);
            if from.exists() {
                let _ = fs::remove_file(&to);
                fs::rename(&from, &to)?;
            }
        }
    }
    let tmp = tmp_path(path);
    {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    set_private(&tmp)?;
    fs::rename(&tmp, path)?;
    set_private(path)?;
    Ok(())
}

pub fn load(path: &Path) -> Result<Vec<u8>> {
    Ok(fs::read(path)?)
}

pub fn write_private_file(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        set_private_dir(parent)?;
    }
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    f.write_all(data)?;
    f.sync_all()?;
    set_private(path)?;
    Ok(())
}

#[cfg(unix)]
pub fn set_private(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn set_private(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_dir(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
    Ok(())
}

#[cfg(not(unix))]
fn set_private_dir(_path: &Path) -> Result<()> {
    Ok(())
}
