//! git fsck 実行処理

use std::path::{Path, PathBuf};
use std::process::Command;

/// 単一リポジトリのfsck結果
#[derive(Debug, Clone)]
pub struct FsckResult {
    pub repo: PathBuf,
    pub success: bool,
    pub code: Option<i32>,
    pub stderr: String,
}

/// fsck集計結果
#[derive(Debug, Default)]
pub struct FsckSummary {
    pub results: Vec<FsckResult>,
}

impl FsckSummary {
    pub fn total(&self) -> usize {
        self.results.len()
    }

    pub fn failed(&self) -> usize {
        self.results.iter().filter(|r| !r.success).count()
    }

    pub fn all_success(&self) -> bool {
        self.failed() == 0
    }
}

/// 単一リポジトリで `git fsck --full` を実行
pub fn run_git_fsck(repo: &Path) -> FsckResult {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("fsck")
        .arg("--full")
        .output();

    match output {
        Ok(out) => FsckResult {
            repo: repo.to_path_buf(),
            success: out.status.success(),
            code: out.status.code(),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        },
        Err(e) => FsckResult {
            repo: repo.to_path_buf(),
            success: false,
            code: None,
            stderr: e.to_string(),
        },
    }
}

/// 複数リポジトリで fsck を実行して集約
pub fn run_fsck_for_repos(repos: &[PathBuf]) -> FsckSummary {
    let mut summary = FsckSummary::default();
    for repo in repos {
        summary.results.push(run_git_fsck(repo));
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_run_git_fsck_success_on_valid_repo() {
        let temp_dir = TempDir::new().unwrap();
        let repo = temp_dir.path().join("repo");
        fs::create_dir_all(&repo).unwrap();

        let init = Command::new("git")
            .arg("init")
            .arg("-q")
            .arg(&repo)
            .status()
            .unwrap();
        assert!(init.success());

        let result = run_git_fsck(&repo);
        assert!(result.success);
        assert_eq!(result.repo, repo);
    }

    #[test]
    fn test_run_git_fsck_failure_on_non_repo() {
        let temp_dir = TempDir::new().unwrap();
        let non_repo = temp_dir.path().join("not-repo");
        fs::create_dir_all(&non_repo).unwrap();

        let result = run_git_fsck(&non_repo);
        assert!(!result.success);
        assert_eq!(result.repo, non_repo);
    }

    #[test]
    fn test_run_fsck_for_repos_summary() {
        let temp_dir = TempDir::new().unwrap();
        let repo = temp_dir.path().join("repo");
        let non_repo = temp_dir.path().join("not-repo");
        fs::create_dir_all(&repo).unwrap();
        fs::create_dir_all(&non_repo).unwrap();

        let init = Command::new("git")
            .arg("init")
            .arg("-q")
            .arg(&repo)
            .status()
            .unwrap();
        assert!(init.success());

        let summary = run_fsck_for_repos(&[repo.clone(), non_repo.clone()]);
        assert_eq!(summary.total(), 2);
        assert_eq!(summary.failed(), 1);
        assert!(!summary.all_success());
        assert!(summary.results.iter().any(|r| r.repo == repo && r.success));
        assert!(summary.results.iter().any(|r| r.repo == non_repo && !r.success));
    }
}
