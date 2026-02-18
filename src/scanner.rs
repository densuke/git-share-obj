//! Gitオブジェクトファイルの探索

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Gitオブジェクトファイルの情報
#[derive(Debug, Clone)]
pub struct GitObjectInfo {
    /// ファイルパス
    pub path: PathBuf,
    /// ハッシュ値 (ディレクトリ名 + ファイル名)
    pub hash: String,
    /// ファイルの作成時刻
    pub created: SystemTime,
    /// ファイルサイズ (バイト)
    pub size: u64,
}

impl GitObjectInfo {
    /// パスからGitObjectInfoを作成する
    ///
    /// Args:
    ///     path: オブジェクトファイルのパス
    ///
    /// Returns:
    ///     成功時はSome(GitObjectInfo)、失敗時はNone
    pub fn from_path(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_str()?;
        let parent = path.parent()?;
        let dir_name = parent.file_name()?.to_str()?;

        // ハッシュは2文字のディレクトリ名 + 38文字のファイル名 = 40文字
        if dir_name.len() != 2 || file_name.len() != 38 {
            return None;
        }

        // 16進数文字のみで構成されているか確認
        if !dir_name.chars().all(|c| c.is_ascii_hexdigit())
            || !file_name.chars().all(|c| c.is_ascii_hexdigit())
        {
            return None;
        }

        let hash = format!("{}{}", dir_name, file_name);
        let metadata = fs::metadata(path).ok()?;
        let created = metadata.modified().ok()?;
        let size = metadata.len();

        Some(GitObjectInfo {
            path: path.to_path_buf(),
            hash,
            created,
            size,
        })
    }
}

/// 指定ディレクトリ以下の全ての.git/objectsを探索する
///
/// Args:
///     base_path: 探索開始ディレクトリ
///
/// Returns:
///     発見した全てのGitオブジェクト情報のベクタ
pub fn scan_git_objects(base_path: &Path) -> Vec<GitObjectInfo> {
    let mut objects = Vec::new();

    // base_path以下の全ての.gitディレクトリを探索
    for entry in WalkDir::new(base_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // .git/objectsディレクトリを発見したら、その中を探索
        if path.ends_with(".git/objects") && path.is_dir() {
            objects.extend(scan_objects_dir(path));
        }
    }

    objects
}

/// 重複ファイルのグループ
#[derive(Debug)]
pub struct DuplicateGroup {
    /// 基準ファイル (最古のタイムスタンプ)
    pub source: GitObjectInfo,
    /// 重複ファイルのリスト (ハードリンクに置換する対象)
    pub duplicates: Vec<GitObjectInfo>,
}

/// オブジェクトファイルを同一ハッシュでグループ化し、重複グループを返す
///
/// Args:
///     objects: 探索で発見したオブジェクト情報のリスト
///
/// Returns:
///     2つ以上のファイルが存在するグループのみ返す
pub fn find_duplicates(objects: Vec<GitObjectInfo>) -> Vec<DuplicateGroup> {
    // ハッシュ値でグループ化
    let mut groups: HashMap<String, Vec<GitObjectInfo>> = HashMap::new();
    for obj in objects {
        groups.entry(obj.hash.clone()).or_default().push(obj);
    }

    // 2つ以上のファイルがあるグループのみ抽出
    groups
        .into_values()
        .filter(|v| v.len() >= 2)
        .map(|mut files| {
            // タイムスタンプでソート (最古が先頭)
            files.sort_by_key(|f| f.created);
            let source = files.remove(0);
            DuplicateGroup {
                source,
                duplicates: files,
            }
        })
        .collect()
}

/// .git/objectsディレクトリ内のオブジェクトファイルを探索する
///
/// Args:
///     objects_dir: .git/objectsディレクトリのパス
///
/// Returns:
///     発見したGitオブジェクト情報のベクタ
fn scan_objects_dir(objects_dir: &Path) -> Vec<GitObjectInfo> {
    let mut objects = Vec::new();

    for entry in WalkDir::new(objects_dir)
        .min_depth(2)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // pack, infoディレクトリは除外
        if let Some(parent) = path.parent() {
            if let Some(parent_name) = parent.file_name() {
                let name = parent_name.to_string_lossy();
                if name == "pack" || name == "info" {
                    continue;
                }
            }
        }

        if path.is_file() {
            if let Some(info) = GitObjectInfo::from_path(path) {
                objects.push(info);
            }
        }
    }

    objects
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    /// テスト用の.git/objects構造を作成する
    fn create_test_git_structure(temp_dir: &Path) -> PathBuf {
        let git_objects = temp_dir.join(".git/objects");
        fs::create_dir_all(&git_objects).unwrap();

        // 有効なオブジェクトファイルを作成 (2文字ディレクトリ + 38文字ファイル)
        let obj_dir = git_objects.join("ab");
        fs::create_dir_all(&obj_dir).unwrap();
        let obj_file = obj_dir.join("cdef1234567890abcdef1234567890abcdef12");
        File::create(&obj_file).unwrap().write_all(b"test").unwrap();

        git_objects
    }

    #[test]
    fn test_git_object_info_from_path_valid() {
        let temp_dir = TempDir::new().unwrap();
        let git_objects = create_test_git_structure(temp_dir.path());

        let obj_path = git_objects.join("ab/cdef1234567890abcdef1234567890abcdef12");
        let info = GitObjectInfo::from_path(&obj_path);

        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.hash, "abcdef1234567890abcdef1234567890abcdef12");
        // "test"は4バイト
        assert_eq!(info.size, 4);
    }

    #[test]
    fn test_git_object_info_from_path_invalid_dir_length() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_dir = temp_dir.path().join("abc"); // 3文字 (無効)
        fs::create_dir_all(&invalid_dir).unwrap();
        let file_path = invalid_dir.join("def1234567890abcdef1234567890abcdef12");
        File::create(&file_path).unwrap();

        let info = GitObjectInfo::from_path(&file_path);
        assert!(info.is_none());
    }

    #[test]
    fn test_git_object_info_from_path_invalid_file_length() {
        let temp_dir = TempDir::new().unwrap();
        let valid_dir = temp_dir.path().join("ab");
        fs::create_dir_all(&valid_dir).unwrap();
        let file_path = valid_dir.join("short"); // 38文字未満 (無効)
        File::create(&file_path).unwrap();

        let info = GitObjectInfo::from_path(&file_path);
        assert!(info.is_none());
    }

    #[test]
    fn test_git_object_info_from_path_invalid_hex() {
        let temp_dir = TempDir::new().unwrap();
        let valid_dir = temp_dir.path().join("zz"); // 非16進数 (無効)
        fs::create_dir_all(&valid_dir).unwrap();
        let file_path = valid_dir.join("cdef1234567890abcdef1234567890abcdef12");
        File::create(&file_path).unwrap();

        let info = GitObjectInfo::from_path(&file_path);
        assert!(info.is_none());
    }

    #[test]
    fn test_scan_git_objects() {
        let temp_dir = TempDir::new().unwrap();
        create_test_git_structure(temp_dir.path());

        let objects = scan_git_objects(temp_dir.path());

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].hash, "abcdef1234567890abcdef1234567890abcdef12");
    }

    #[test]
    fn test_scan_git_objects_excludes_pack() {
        let temp_dir = TempDir::new().unwrap();
        let git_objects = temp_dir.path().join(".git/objects");

        // packディレクトリを作成
        let pack_dir = git_objects.join("pack");
        fs::create_dir_all(&pack_dir).unwrap();
        File::create(pack_dir.join("pack-abc123.pack")).unwrap();

        // 有効なオブジェクトも作成
        let obj_dir = git_objects.join("ab");
        fs::create_dir_all(&obj_dir).unwrap();
        File::create(obj_dir.join("cdef1234567890abcdef1234567890abcdef12")).unwrap();

        let objects = scan_git_objects(temp_dir.path());

        assert_eq!(objects.len(), 1);
    }

    #[test]
    fn test_scan_git_objects_excludes_info() {
        let temp_dir = TempDir::new().unwrap();
        let git_objects = temp_dir.path().join(".git/objects");

        // infoディレクトリを作成
        let info_dir = git_objects.join("info");
        fs::create_dir_all(&info_dir).unwrap();
        File::create(info_dir.join("packs")).unwrap();

        // 有効なオブジェクトも作成
        let obj_dir = git_objects.join("ab");
        fs::create_dir_all(&obj_dir).unwrap();
        File::create(obj_dir.join("cdef1234567890abcdef1234567890abcdef12")).unwrap();

        let objects = scan_git_objects(temp_dir.path());

        assert_eq!(objects.len(), 1);
    }

    #[test]
    fn test_scan_multiple_git_repos() {
        let temp_dir = TempDir::new().unwrap();

        // 複数のgitリポジトリを作成
        for repo in ["repo1", "repo2"] {
            let git_objects = temp_dir.path().join(repo).join(".git/objects/ab");
            fs::create_dir_all(&git_objects).unwrap();
            File::create(git_objects.join("cdef1234567890abcdef1234567890abcdef12")).unwrap();
        }

        let objects = scan_git_objects(temp_dir.path());

        assert_eq!(objects.len(), 2);
    }

    #[test]
    fn test_find_duplicates_no_duplicates() {
        let temp_dir = TempDir::new().unwrap();

        // 異なるハッシュのファイルを作成
        for (dir, file) in [
            ("ab", "cdef1234567890abcdef1234567890abcdef12"),
            ("cd", "ef12345678901234567890123456789012abcd"),
        ] {
            let obj_dir = temp_dir.path().join(".git/objects").join(dir);
            fs::create_dir_all(&obj_dir).unwrap();
            File::create(obj_dir.join(file)).unwrap();
        }

        let objects = scan_git_objects(temp_dir.path());
        let duplicates = find_duplicates(objects);

        assert!(duplicates.is_empty());
    }

    #[test]
    fn test_find_duplicates_with_duplicates() {
        let temp_dir = TempDir::new().unwrap();

        // 2つのリポジトリに同じハッシュのファイルを作成
        for repo in ["repo1", "repo2"] {
            let obj_dir = temp_dir.path().join(repo).join(".git/objects/ab");
            fs::create_dir_all(&obj_dir).unwrap();
            let file = obj_dir.join("cdef1234567890abcdef1234567890abcdef12");
            File::create(&file).unwrap();
            // タイムスタンプをずらすために少し待つ
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let objects = scan_git_objects(temp_dir.path());
        let duplicates = find_duplicates(objects);

        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].duplicates.len(), 1);
    }

    #[test]
    fn test_find_duplicates_oldest_is_source() {
        let temp_dir = TempDir::new().unwrap();

        // 3つのリポジトリに同じハッシュのファイルを作成
        let mut paths = Vec::new();
        for repo in ["repo1", "repo2", "repo3"] {
            let obj_dir = temp_dir.path().join(repo).join(".git/objects/ab");
            fs::create_dir_all(&obj_dir).unwrap();
            let file = obj_dir.join("cdef1234567890abcdef1234567890abcdef12");
            File::create(&file).unwrap();
            paths.push(file);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let objects = scan_git_objects(temp_dir.path());
        let duplicates = find_duplicates(objects);

        assert_eq!(duplicates.len(), 1);
        // 最古のファイルがsourceになっているか確認
        assert_eq!(duplicates[0].duplicates.len(), 2);
        // sourceは最も古いタイムスタンプを持つ
        for dup in &duplicates[0].duplicates {
            assert!(duplicates[0].source.created <= dup.created);
        }
    }
}
