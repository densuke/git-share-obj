//! コマンドライン引数のパースと設定

use clap::Parser;

/// Gitオブジェクトの重複ファイルをハードリンクで共有するツール
#[derive(Parser, Debug)]
#[command(name = "git-share-obj")]
#[command(version, about, long_about = None)]
pub struct Args {
    /// 探索対象のディレクトリ (複数指定可能、デフォルト: カレントディレクトリ)
    #[arg(default_values_t = vec![String::from(".")])]
    pub paths: Vec<String>,

    /// ドライラン (実際には変更せず、検出結果のみ表示)
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,

    /// 詳細出力
    #[arg(short, long)]
    pub verbose: bool,
}

impl Args {
    /// 引数をパースして返す
    pub fn parse_args() -> Self {
        Args::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = Args::parse_from(["git-share-obj"]);
        assert_eq!(args.paths, vec!["."]);
        assert!(!args.dry_run);
        assert!(!args.verbose);
    }

    #[test]
    fn test_dry_run_short() {
        let args = Args::parse_from(["git-share-obj", "-n"]);
        assert!(args.dry_run);
    }

    #[test]
    fn test_dry_run_long() {
        let args = Args::parse_from(["git-share-obj", "--dry-run"]);
        assert!(args.dry_run);
    }

    #[test]
    fn test_verbose_short() {
        let args = Args::parse_from(["git-share-obj", "-v"]);
        assert!(args.verbose);
    }

    #[test]
    fn test_verbose_long() {
        let args = Args::parse_from(["git-share-obj", "--verbose"]);
        assert!(args.verbose);
    }

    #[test]
    fn test_single_path() {
        let args = Args::parse_from(["git-share-obj", "/path/to/dir"]);
        assert_eq!(args.paths, vec!["/path/to/dir"]);
    }

    #[test]
    fn test_multiple_paths() {
        let args = Args::parse_from(["git-share-obj", "/path/a", "/path/b", "/path/c"]);
        assert_eq!(args.paths, vec!["/path/a", "/path/b", "/path/c"]);
    }

    #[test]
    fn test_all_options_single_path() {
        let args = Args::parse_from(["git-share-obj", "-n", "-v", "/custom/path"]);
        assert!(args.dry_run);
        assert!(args.verbose);
        assert_eq!(args.paths, vec!["/custom/path"]);
    }

    #[test]
    fn test_all_options_multiple_paths() {
        let args = Args::parse_from(["git-share-obj", "-n", "-v", "/path/a", "/path/b"]);
        assert!(args.dry_run);
        assert!(args.verbose);
        assert_eq!(args.paths, vec!["/path/a", "/path/b"]);
    }
}
