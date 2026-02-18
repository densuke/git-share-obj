//! ハードリンク処理

use std::path::Path;

/// 2つのパスが同一ファイルシステム上にあるか確認する
pub fn is_same_filesystem(_path1: &Path, _path2: &Path) -> bool {
    // TODO: 実装
    true
}

/// ファイルをハードリンクに置換する
pub fn replace_with_hardlink(_source: &Path, _target: &Path) -> std::io::Result<()> {
    // TODO: 実装
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
