//! コマンドライン引数のパースと設定

use clap::Parser;

/// Gitオブジェクトの重複ファイルをハードリンクで共有するツール
#[derive(Parser, Debug)]
#[command(name = "git-share-obj")]
#[command(version, about, long_about = None)]
pub struct Args {
    /// 探索対象のディレクトリ (デフォルト: カレントディレクトリ)
    #[arg(default_value = ".")]
    pub path: String,

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
        assert_eq!(args.path, ".");
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
    fn test_custom_path() {
        let args = Args::parse_from(["git-share-obj", "/path/to/dir"]);
        assert_eq!(args.path, "/path/to/dir");
    }

    #[test]
    fn test_all_options() {
        let args = Args::parse_from(["git-share-obj", "-n", "-v", "/custom/path"]);
        assert!(args.dry_run);
        assert!(args.verbose);
        assert_eq!(args.path, "/custom/path");
    }
}
