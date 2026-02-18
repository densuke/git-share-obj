//! リポジトリロック処理（lock file + OS advisory lock）

use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

/// リポジトリロック獲得結果
#[derive(Debug)]
pub enum LockError {
    LockPathCreateFailed(String),
    LockFileOpenFailed(String),
    LockBusy(String),
}

/// 獲得済みロック
#[derive(Debug)]
pub struct RepoLock {
    pub repo: PathBuf,
    pub lock_path: PathBuf,
    file: File,
}

impl Drop for RepoLock {
    fn drop(&mut self) {
        let _ = unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}

/// ロックファイルパスを返す
pub fn lock_file_path(repo: &Path) -> PathBuf {
    repo.join(".git").join("objects").join("git-share-obj.lock")
}

/// 単一リポジトリのロックを試行
pub fn try_lock_repo(repo: &Path) -> Result<RepoLock, LockError> {
    let lock_path = lock_file_path(repo);
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| LockError::LockPathCreateFailed(e.to_string()))?;
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&lock_path)
        .map_err(|e| LockError::LockFileOpenFailed(e.to_string()))?;

    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc != 0 {
        return Err(LockError::LockBusy(format!("{}", lock_path.display())));
    }

    Ok(RepoLock {
        repo: repo.to_path_buf(),
        lock_path,
        file,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_repo(dir: &Path) {
        let status = Command::new("git")
            .arg("init")
            .arg("-q")
            .arg(dir)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn test_try_lock_repo_success() {
        let temp_dir = TempDir::new().unwrap();
        let repo = temp_dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_repo(&repo);

        let lock = try_lock_repo(&repo);
        assert!(lock.is_ok());
    }

    #[test]
    fn test_try_lock_repo_busy_when_locked_twice() {
        let temp_dir = TempDir::new().unwrap();
        let repo = temp_dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_repo(&repo);

        let _lock1 = try_lock_repo(&repo).unwrap();
        let lock2 = try_lock_repo(&repo);
        assert!(matches!(lock2, Err(LockError::LockBusy(_))));
    }

    #[test]
    fn test_try_lock_repo_can_reacquire_after_drop() {
        let temp_dir = TempDir::new().unwrap();
        let repo = temp_dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_repo(&repo);

        {
            let _lock = try_lock_repo(&repo).unwrap();
        }

        let lock2 = try_lock_repo(&repo);
        assert!(lock2.is_ok());
    }
}
