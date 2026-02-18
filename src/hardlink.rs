//! ハードリンク処理

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// ハードリンク置換の結果
#[derive(Debug, PartialEq)]
pub enum ReplaceResult {
    /// 置換成功
    Replaced,
    /// 既にハードリンク済み (inode番号が同一)
    AlreadyLinked,
    /// ファイルシステムが異なるためスキップ
    CrossFilesystem,
    /// ハードリンク作成失敗後にロールバック成功
    RolledBack(String),
    /// ハードリンク作成失敗後のロールバックも失敗
    RollbackFailed(String),
    /// エラー発生
    Error(String),
}

/// 2つのパスが同一ファイルシステム上にあるか確認する
///
/// Args:
///     path1: 比較対象のパス1
///     path2: 比較対象のパス2
///
/// Returns:
///     同一ファイルシステム上ならtrue
#[cfg(unix)]
pub fn is_same_filesystem(path1: &Path, path2: &Path) -> io::Result<bool> {
    let meta1 = fs::metadata(path1)?;
    let meta2 = fs::metadata(path2)?;
    Ok(meta1.dev() == meta2.dev())
}

#[cfg(not(unix))]
pub fn is_same_filesystem(_path1: &Path, _path2: &Path) -> io::Result<bool> {
    // 非Unix環境ではハードリンクをサポートしない
    Ok(false)
}

/// 2つのファイルが既に同一inode (ハードリンク済み) か確認する
///
/// Args:
///     path1: 比較対象のパス1
///     path2: 比較対象のパス2
///
/// Returns:
///     同一inodeならtrue
#[cfg(unix)]
pub fn is_same_inode(path1: &Path, path2: &Path) -> io::Result<bool> {
    let meta1 = fs::metadata(path1)?;
    let meta2 = fs::metadata(path2)?;
    Ok(meta1.dev() == meta2.dev() && meta1.ino() == meta2.ino())
}

#[cfg(not(unix))]
pub fn is_same_inode(_path1: &Path, _path2: &Path) -> io::Result<bool> {
    Ok(false)
}

/// ファイルをハードリンクに置換する
///
/// Args:
///     source: 基準ファイル (リンク元)
///     target: 置換対象ファイル (削除してハードリンクに置き換える)
///
/// Returns:
///     置換結果
pub fn replace_with_hardlink(source: &Path, target: &Path) -> ReplaceResult {
    // ファイルシステムの確認
    match is_same_filesystem(source, target) {
        Ok(true) => {}
        Ok(false) => return ReplaceResult::CrossFilesystem,
        Err(e) => return ReplaceResult::Error(e.to_string()),
    }

    // 既にハードリンク済みか確認
    match is_same_inode(source, target) {
        Ok(true) => return ReplaceResult::AlreadyLinked,
        Ok(false) => {}
        Err(e) => return ReplaceResult::Error(e.to_string()),
    }

    let backup = backup_path(target);
    if let Err(e) = fs::rename(target, &backup) {
        return ReplaceResult::Error(format!("退避リネーム失敗: {}", e));
    }

    if let Err(e) = fs::hard_link(source, target) {
        remove_if_regular_file(target);
        return match fs::rename(&backup, target) {
            Ok(()) => ReplaceResult::RolledBack(format!(
                "ハードリンク作成失敗: {} (ロールバック成功)",
                e
            )),
            Err(rollback_err) => ReplaceResult::RollbackFailed(format!(
                "ハードリンク作成失敗: {} (ロールバック失敗: {})",
                e, rollback_err
            )),
        };
    }

    if let Err(e) = fs::remove_file(&backup) {
        return ReplaceResult::Error(format!(
            "退避ファイル削除失敗: {} (退避ファイル: {})",
            e,
            backup.display()
        ));
    }

    ReplaceResult::Replaced
}

fn backup_path(target: &Path) -> PathBuf {
    let file_name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "target".to_string());
    target.with_file_name(format!("{}.git-share-obj.bak", file_name))
}

fn remove_if_regular_file(path: &Path) {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => {
            let _ = fs::remove_file(path);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_is_same_filesystem_same_dir() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1");
        let file2 = temp_dir.path().join("file2");
        File::create(&file1).unwrap();
        File::create(&file2).unwrap();

        assert!(is_same_filesystem(&file1, &file2).unwrap());
    }

    #[test]
    fn test_is_same_inode_different_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1");
        let file2 = temp_dir.path().join("file2");
        File::create(&file1).unwrap();
        File::create(&file2).unwrap();

        assert!(!is_same_inode(&file1, &file2).unwrap());
    }

    #[test]
    fn test_is_same_inode_hardlinked_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1");
        let file2 = temp_dir.path().join("file2");
        File::create(&file1).unwrap().write_all(b"test").unwrap();
        fs::hard_link(&file1, &file2).unwrap();

        assert!(is_same_inode(&file1, &file2).unwrap());
    }

    #[test]
    fn test_replace_with_hardlink_success() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source");
        let target = temp_dir.path().join("target");

        File::create(&source).unwrap().write_all(b"source content").unwrap();
        File::create(&target).unwrap().write_all(b"target content").unwrap();

        let result = replace_with_hardlink(&source, &target);
        assert_eq!(result, ReplaceResult::Replaced);

        // ハードリンクが作成されたことを確認
        assert!(is_same_inode(&source, &target).unwrap());
        assert!(!temp_dir.path().join("target.git-share-obj.bak").exists());
    }

    #[test]
    fn test_replace_with_hardlink_already_linked() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source");
        let target = temp_dir.path().join("target");

        File::create(&source).unwrap().write_all(b"content").unwrap();
        fs::hard_link(&source, &target).unwrap();

        let result = replace_with_hardlink(&source, &target);
        assert_eq!(result, ReplaceResult::AlreadyLinked);
    }

    #[test]
    fn test_replace_with_hardlink_nonexistent_source() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("nonexistent");
        let target = temp_dir.path().join("target");

        File::create(&target).unwrap();

        let result = replace_with_hardlink(&source, &target);
        assert!(matches!(result, ReplaceResult::Error(_)));
    }

}
