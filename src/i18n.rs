//! 国際化 (i18n) サポート

use sys_locale::get_locale;

/// 現在のロケールが日本語かどうかを判定する
pub fn is_japanese() -> bool {
    get_locale()
        .map(|l| l.starts_with("ja"))
        .unwrap_or(false)
}

/// バイト数を人間が読みやすい形式にフォーマットする
///
/// Args:
///     bytes: バイト数
///
/// Returns:
///     フォーマットされた文字列 (例: "1.5 MB")
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// メッセージキー
#[derive(Clone, Copy)]
pub enum Msg {
    // ヘルプ関連
    AppDescription,
    ArgPaths,
    ArgDryRun,
    ArgVerbose,

    // 処理中メッセージ
    Scanning,
    FoundObjects,
    FoundDuplicateGroups,
    DuplicateFiles,
    Processing,
    ProcessingDevice,
    DeviceGroups,

    // 結果メッセージ
    Replaced,
    AlreadyLinked,
    CrossFilesystem,
    ErrorOccurred,

    // サマリー
    SummaryDryRun,
    SummaryComplete,
    TotalDuplicates,
    TotalReplaced,
    TotalSkipped,
    TotalErrors,

    // 削減容量
    GroupSavings,
    EstimatedSavings,
    TotalSavings,

    // fsck
    FsckRunning,
    FsckOk,
    FsckFailed,
    FsckSummary,
    FsckOnlyComplete,
    FsckSkipped,
    AbortOnFsckFailure,

    // rollback
    RollbackOccurred,
    RollbackFailed,
}

/// ローカライズされたメッセージを取得する
pub fn msg(key: Msg) -> &'static str {
    if is_japanese() {
        msg_ja(key)
    } else {
        msg_en(key)
    }
}

fn msg_ja(key: Msg) -> &'static str {
    match key {
        // ヘルプ関連
        Msg::AppDescription => "Gitオブジェクトの重複ファイルをハードリンクで共有するツール",
        Msg::ArgPaths => "探索対象のディレクトリ (複数指定可能、デフォルト: カレントディレクトリ)",
        Msg::ArgDryRun => "ドライラン (実際には変更せず、検出結果のみ表示)",
        Msg::ArgVerbose => "詳細出力",

        // 処理中メッセージ
        Msg::Scanning => "探索中...",
        Msg::FoundObjects => "オブジェクトファイル発見",
        Msg::FoundDuplicateGroups => "重複グループ発見",
        Msg::DuplicateFiles => "重複ファイル",
        Msg::Processing => "処理中...",
        Msg::ProcessingDevice => "デバイス処理中",
        Msg::DeviceGroups => "デバイスグループ",

        // 結果メッセージ
        Msg::Replaced => "置換完了",
        Msg::AlreadyLinked => "既にリンク済み",
        Msg::CrossFilesystem => "ファイルシステム跨ぎのためスキップ",
        Msg::ErrorOccurred => "エラー",

        // サマリー
        Msg::SummaryDryRun => "=== ドライラン結果 ===",
        Msg::SummaryComplete => "=== 処理完了 ===",
        Msg::TotalDuplicates => "重複ファイル総数",
        Msg::TotalReplaced => "置換成功",
        Msg::TotalSkipped => "スキップ",
        Msg::TotalErrors => "エラー",

        // 削減容量
        Msg::GroupSavings => "グループ削減容量",
        Msg::EstimatedSavings => "見込み削減容量",
        Msg::TotalSavings => "合計削減容量",

        // fsck
        Msg::FsckRunning => "fsck実行中",
        Msg::FsckOk => "fsck成功",
        Msg::FsckFailed => "fsck失敗",
        Msg::FsckSummary => "fsck集計",
        Msg::FsckOnlyComplete => "=== fsckのみ完了 ===",
        Msg::FsckSkipped => "fsckスキップ (--no-fsck)",
        Msg::AbortOnFsckFailure => "fsck失敗のため置換処理を中止",

        // rollback
        Msg::RollbackOccurred => "ロールバック",
        Msg::RollbackFailed => "ロールバック失敗",
    }
}

fn msg_en(key: Msg) -> &'static str {
    match key {
        // Help
        Msg::AppDescription => "Share duplicate Git objects using hard links",
        Msg::ArgPaths => "Target directories (multiple allowed, default: current directory)",
        Msg::ArgDryRun => "Dry run (only show results without making changes)",
        Msg::ArgVerbose => "Verbose output",

        // Processing
        Msg::Scanning => "Scanning...",
        Msg::FoundObjects => "object files found",
        Msg::FoundDuplicateGroups => "duplicate groups found",
        Msg::DuplicateFiles => "duplicate files",
        Msg::Processing => "Processing...",
        Msg::ProcessingDevice => "Processing device",
        Msg::DeviceGroups => "device groups",

        // Results
        Msg::Replaced => "Replaced",
        Msg::AlreadyLinked => "Already linked",
        Msg::CrossFilesystem => "Skipped (cross-filesystem)",
        Msg::ErrorOccurred => "Error",

        // Summary
        Msg::SummaryDryRun => "=== Dry Run Results ===",
        Msg::SummaryComplete => "=== Complete ===",
        Msg::TotalDuplicates => "Total duplicates",
        Msg::TotalReplaced => "Replaced",
        Msg::TotalSkipped => "Skipped",
        Msg::TotalErrors => "Errors",

        // Savings
        Msg::GroupSavings => "Group savings",
        Msg::EstimatedSavings => "Estimated savings",
        Msg::TotalSavings => "Total savings",

        // fsck
        Msg::FsckRunning => "Running fsck",
        Msg::FsckOk => "fsck ok",
        Msg::FsckFailed => "fsck failed",
        Msg::FsckSummary => "fsck summary",
        Msg::FsckOnlyComplete => "=== fsck-only complete ===",
        Msg::FsckSkipped => "fsck skipped (--no-fsck)",
        Msg::AbortOnFsckFailure => "Aborting replacement due to fsck failure",

        // rollback
        Msg::RollbackOccurred => "Rollback",
        Msg::RollbackFailed => "Rollback failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msg_returns_string() {
        // メッセージが空でないことを確認
        assert!(!msg_ja(Msg::AppDescription).is_empty());
        assert!(!msg_en(Msg::AppDescription).is_empty());
    }

    #[test]
    fn test_all_keys_have_translations() {
        // 全てのキーに対応する翻訳があることを確認
        let keys = [
            Msg::AppDescription,
            Msg::ArgPaths,
            Msg::ArgDryRun,
            Msg::ArgVerbose,
            Msg::Scanning,
            Msg::FoundObjects,
            Msg::FoundDuplicateGroups,
            Msg::DuplicateFiles,
            Msg::Processing,
            Msg::ProcessingDevice,
            Msg::DeviceGroups,
            Msg::Replaced,
            Msg::AlreadyLinked,
            Msg::CrossFilesystem,
            Msg::ErrorOccurred,
            Msg::SummaryDryRun,
            Msg::SummaryComplete,
            Msg::TotalDuplicates,
            Msg::TotalReplaced,
            Msg::TotalSkipped,
            Msg::TotalErrors,
            Msg::GroupSavings,
            Msg::EstimatedSavings,
            Msg::TotalSavings,
            Msg::FsckRunning,
            Msg::FsckOk,
            Msg::FsckFailed,
            Msg::FsckSummary,
            Msg::FsckOnlyComplete,
            Msg::FsckSkipped,
            Msg::AbortOnFsckFailure,
            Msg::RollbackOccurred,
            Msg::RollbackFailed,
        ];

        for key in keys {
            assert!(!msg_ja(key).is_empty());
            assert!(!msg_en(key).is_empty());
        }
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1024 * 1023), "1023.00 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 100), "100.00 MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 2), "2.00 GB");
    }
}
