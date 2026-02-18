//! Gitオブジェクトファイルの探索

use std::path::Path;

/// .git/objectsディレクトリを探索し、オブジェクトファイルを収集する
pub fn scan_git_objects(_base_path: &Path) -> Vec<std::path::PathBuf> {
    // TODO: 実装
    Vec::new()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
