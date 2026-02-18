//! Gitオブジェクトファイルの探索

use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

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
    /// inode番号 (Unix系のみ、ハードリンク検出用)
    pub inode: u64,
    /// デバイスID (Unix系のみ、ファイルシステム識別用)
    pub device: u64,
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

        #[cfg(unix)]
        let (inode, device) = (metadata.ino(), metadata.dev());

        #[cfg(not(unix))]
        let (inode, device) = (0, 0);

        Some(GitObjectInfo {
            path: path.to_path_buf(),
            hash,
            created,
            size,
            inode,
            device,
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

/// 指定ディレクトリ以下のGitリポジトリルートを列挙する
///
/// `.git/objects` が存在するディレクトリをGitリポジトリとして扱い、
/// リポジトリルート（`.git` の親ディレクトリ）を重複なく返す。
pub fn find_git_repositories(base_path: &Path) -> Vec<PathBuf> {
    let mut repos = HashSet::new();

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.ends_with(".git/objects") && path.is_dir() {
            if let Some(git_dir) = path.parent() {
                if let Some(repo_root) = git_dir.parent() {
                    repos.insert(repo_root.to_path_buf());
                }
            }
        }
    }

    let mut repo_list: Vec<_> = repos.into_iter().collect();
    repo_list.sort();
    repo_list
}

/// 重複ファイルのグループ
#[derive(Debug)]
pub struct DuplicateGroup {
    /// 基準ファイル (既存ハードリンクグループの代表、または最古のファイル)
    pub source: GitObjectInfo,
    /// 重複ファイルのリスト (ハードリンクに置換する対象、既にリンク済みは含まない)
    pub duplicates: Vec<GitObjectInfo>,
}

/// オブジェクトファイルをデバイスIDでグループ化する
///
/// 異なるデバイス上のファイルはハードリンクできないため、
/// デバイスごとに分けて処理する必要がある。
///
/// Args:
///     objects: 探索で発見したオブジェクト情報のリスト
///
/// Returns:
///     デバイスIDをキーとしたHashMap
pub fn group_by_device(objects: Vec<GitObjectInfo>) -> HashMap<u64, Vec<GitObjectInfo>> {
    let mut groups: HashMap<u64, Vec<GitObjectInfo>> = HashMap::new();
    for obj in objects {
        groups.entry(obj.device).or_default().push(obj);
    }
    groups
}

/// オブジェクトファイルを同一ハッシュでグループ化し、重複グループを返す
///
/// 既存のハードリンクグループがある場合は、そのグループを優先してsourceとする。
/// これにより、後から古いファイルが追加されても既存のハードリンクを壊さない。
///
/// Args:
///     objects: 探索で発見したオブジェクト情報のリスト
///
/// Returns:
///     2つ以上のファイルが存在し、かつ未リンクファイルがあるグループのみ返す
pub fn find_duplicates(objects: Vec<GitObjectInfo>) -> Vec<DuplicateGroup> {
    // ハッシュ値でグループ化
    let mut groups: HashMap<String, Vec<GitObjectInfo>> = HashMap::new();
    for obj in objects {
        groups.entry(obj.hash.clone()).or_default().push(obj);
    }

    // 2つ以上のファイルがあるグループを処理
    groups
        .into_values()
        .filter(|v| v.len() >= 2)
        .filter_map(|files| select_source_and_duplicates(files))
        .collect()
}

/// グループ内からsourceと未リンクのduplicatesを選定する
///
/// 1. 同一inode (同一デバイス上) のファイルをサブグループ化
/// 2. 最大のサブグループ (最も多くリンクされている) のファイルをsource候補
/// 3. source候補の中から1つを選び、他のサブグループのファイルをduplicatesに
fn select_source_and_duplicates(files: Vec<GitObjectInfo>) -> Option<DuplicateGroup> {
    // (device, inode) でサブグループ化
    let mut inode_groups: HashMap<(u64, u64), Vec<GitObjectInfo>> = HashMap::new();
    for file in files {
        inode_groups
            .entry((file.device, file.inode))
            .or_default()
            .push(file);
    }

    // 最大のサブグループを見つける (同数なら最初に見つかったもの)
    let (source_key, _) = inode_groups
        .iter()
        .max_by_key(|(_, group)| group.len())?;

    let source_key = *source_key;

    // sourceグループから1つを選ぶ (最古のもの)
    let mut source_candidates: Vec<_> = inode_groups
        .remove(&source_key)
        .unwrap_or_default();
    source_candidates.sort_by_key(|f| f.created);
    let source = source_candidates.into_iter().next()?;

    // 他のサブグループのファイルをduplicatesとして収集
    let duplicates: Vec<_> = inode_groups
        .into_values()
        .flatten()
        .collect();

    // 置換対象がなければNone
    if duplicates.is_empty() {
        return None;
    }

    Some(DuplicateGroup { source, duplicates })
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
    fn test_find_duplicates_all_independent_files() {
        let temp_dir = TempDir::new().unwrap();

        // 3つのリポジトリに同じハッシュのファイルを作成 (全て独立)
        for repo in ["repo1", "repo2", "repo3"] {
            let obj_dir = temp_dir.path().join(repo).join(".git/objects/ab");
            fs::create_dir_all(&obj_dir).unwrap();
            let file = obj_dir.join("cdef1234567890abcdef1234567890abcdef12");
            File::create(&file).unwrap().write_all(b"test").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let objects = scan_git_objects(temp_dir.path());
        let duplicates = find_duplicates(objects);

        assert_eq!(duplicates.len(), 1);
        // 全て独立なので、1つがsource、残り2つがduplicates
        assert_eq!(duplicates[0].duplicates.len(), 2);
    }

    #[test]
    fn test_find_duplicates_existing_hardlink_is_source() {
        let temp_dir = TempDir::new().unwrap();

        // repo1とrepo2に同じハッシュのファイルを作成 (repo1が古い)
        let obj_dir1 = temp_dir.path().join("repo1/.git/objects/ab");
        fs::create_dir_all(&obj_dir1).unwrap();
        let file1 = obj_dir1.join("cdef1234567890abcdef1234567890abcdef12");
        File::create(&file1).unwrap().write_all(b"test").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let obj_dir2 = temp_dir.path().join("repo2/.git/objects/ab");
        fs::create_dir_all(&obj_dir2).unwrap();
        let file2 = obj_dir2.join("cdef1234567890abcdef1234567890abcdef12");
        File::create(&file2).unwrap().write_all(b"test").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // repo2とrepo3をハードリンクにする (repo3が最も新しいが、repo2とリンク済み)
        let obj_dir3 = temp_dir.path().join("repo3/.git/objects/ab");
        fs::create_dir_all(&obj_dir3).unwrap();
        let file3 = obj_dir3.join("cdef1234567890abcdef1234567890abcdef12");
        fs::hard_link(&file2, &file3).unwrap();

        let objects = scan_git_objects(temp_dir.path());
        let duplicates = find_duplicates(objects);

        // repo1は古いが単独、repo2/repo3はハードリンク済み
        // よってrepo2/repo3グループがsourceになり、repo1がduplicateになる
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].duplicates.len(), 1);

        // sourceはrepo2またはrepo3 (ハードリンクグループ)
        let source_path = duplicates[0].source.path.to_string_lossy();
        assert!(
            source_path.contains("repo2") || source_path.contains("repo3"),
            "sourceはハードリンクグループから選ばれるべき: {}",
            source_path
        );

        // duplicateはrepo1 (単独ファイル)
        let dup_path = duplicates[0].duplicates[0].path.to_string_lossy();
        assert!(
            dup_path.contains("repo1"),
            "duplicateは単独ファイルのrepo1であるべき: {}",
            dup_path
        );
    }

    #[test]
    fn test_find_duplicates_all_already_linked() {
        let temp_dir = TempDir::new().unwrap();

        // 全てのファイルがハードリンク済みの場合
        let obj_dir1 = temp_dir.path().join("repo1/.git/objects/ab");
        fs::create_dir_all(&obj_dir1).unwrap();
        let file1 = obj_dir1.join("cdef1234567890abcdef1234567890abcdef12");
        File::create(&file1).unwrap().write_all(b"test").unwrap();

        let obj_dir2 = temp_dir.path().join("repo2/.git/objects/ab");
        fs::create_dir_all(&obj_dir2).unwrap();
        let file2 = obj_dir2.join("cdef1234567890abcdef1234567890abcdef12");
        fs::hard_link(&file1, &file2).unwrap();

        let objects = scan_git_objects(temp_dir.path());
        let duplicates = find_duplicates(objects);

        // 全てリンク済みなので、置換対象なし
        assert!(duplicates.is_empty());
    }

    #[test]
    fn test_find_duplicates_add_new_file_to_existing_group() {
        let temp_dir = TempDir::new().unwrap();

        // 既存のハードリンクグループ (repo1, repo2)
        let obj_dir1 = temp_dir.path().join("repo1/.git/objects/ab");
        fs::create_dir_all(&obj_dir1).unwrap();
        let file1 = obj_dir1.join("cdef1234567890abcdef1234567890abcdef12");
        File::create(&file1).unwrap().write_all(b"test").unwrap();

        let obj_dir2 = temp_dir.path().join("repo2/.git/objects/ab");
        fs::create_dir_all(&obj_dir2).unwrap();
        let file2 = obj_dir2.join("cdef1234567890abcdef1234567890abcdef12");
        fs::hard_link(&file1, &file2).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        // 新しいファイルを追加 (repo3, repo4は独立)
        for repo in ["repo3", "repo4"] {
            let obj_dir = temp_dir.path().join(repo).join(".git/objects/ab");
            fs::create_dir_all(&obj_dir).unwrap();
            let file = obj_dir.join("cdef1234567890abcdef1234567890abcdef12");
            File::create(&file).unwrap().write_all(b"test").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let objects = scan_git_objects(temp_dir.path());
        let duplicates = find_duplicates(objects);

        assert_eq!(duplicates.len(), 1);
        // 既存グループ (repo1/repo2) がsource、新規 (repo3/repo4) がduplicates
        assert_eq!(duplicates[0].duplicates.len(), 2);

        let source_path = duplicates[0].source.path.to_string_lossy();
        assert!(
            source_path.contains("repo1") || source_path.contains("repo2"),
            "sourceは既存ハードリンクグループから選ばれるべき: {}",
            source_path
        );
    }

    #[test]
    fn test_group_by_device_single_device() {
        let temp_dir = TempDir::new().unwrap();

        // 同じデバイス上に複数のリポジトリを作成
        for repo in ["repo1", "repo2", "repo3"] {
            let obj_dir = temp_dir.path().join(repo).join(".git/objects/ab");
            fs::create_dir_all(&obj_dir).unwrap();
            File::create(obj_dir.join("cdef1234567890abcdef1234567890abcdef12")).unwrap();
        }

        let objects = scan_git_objects(temp_dir.path());
        let device_groups = group_by_device(objects);

        // 全て同じデバイス上なので1グループ
        assert_eq!(device_groups.len(), 1);
        // 3ファイル全て同じグループ
        let first_group: Vec<_> = device_groups.into_values().next().unwrap();
        assert_eq!(first_group.len(), 3);
    }

    #[test]
    fn test_group_by_device_empty() {
        let objects: Vec<GitObjectInfo> = vec![];
        let device_groups = group_by_device(objects);
        assert!(device_groups.is_empty());
    }

    #[test]
    fn test_find_git_repositories_single() {
        let temp_dir = TempDir::new().unwrap();
        let repo = temp_dir.path().join("repo1");
        fs::create_dir_all(repo.join(".git/objects/ab")).unwrap();
        File::create(repo.join(".git/objects/ab/cdef1234567890abcdef1234567890abcdef12")).unwrap();

        let repos = find_git_repositories(temp_dir.path());
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0], repo);
    }

    #[test]
    fn test_find_git_repositories_multiple_and_unique() {
        let temp_dir = TempDir::new().unwrap();
        let repo1 = temp_dir.path().join("repo1");
        let repo2 = temp_dir.path().join("nested/repo2");

        fs::create_dir_all(repo1.join(".git/objects/ab")).unwrap();
        fs::create_dir_all(repo1.join(".git/objects/cd")).unwrap();
        fs::create_dir_all(repo2.join(".git/objects/ef")).unwrap();

        File::create(repo1.join(".git/objects/ab/cdef1234567890abcdef1234567890abcdef12")).unwrap();
        File::create(repo1.join(".git/objects/cd/ef12345678901234567890123456789012abcd")).unwrap();
        File::create(repo2.join(".git/objects/ef/1234567890abcdef1234567890abcdef123456")).unwrap();

        let repos = find_git_repositories(temp_dir.path());
        assert_eq!(repos.len(), 2);
        assert!(repos.contains(&repo1));
        assert!(repos.contains(&repo2));
    }
}
