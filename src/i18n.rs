//! 国際化 (i18n) サポート

use sys_locale::get_locale;

/// 現在のロケールが日本語かどうかを判定する
pub fn is_japanese() -> bool {
    get_locale()
        .map(|l| l.starts_with("ja"))
        .unwrap_or(false)
}

/// メッセージキー
#[derive(Clone, Copy)]
pub enum Msg {
    // ヘルプ関連
    AppDescription,
    ArgPath,
    ArgDryRun,
    ArgVerbose,

    // 処理中メッセージ
    Scanning,
    FoundObjects,
    FoundDuplicateGroups,
    DuplicateFiles,
    Processing,

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
        Msg::ArgPath => "探索対象のディレクトリ (デフォルト: カレントディレクトリ)",
        Msg::ArgDryRun => "ドライラン (実際には変更せず、検出結果のみ表示)",
        Msg::ArgVerbose => "詳細出力",

        // 処理中メッセージ
        Msg::Scanning => "探索中...",
        Msg::FoundObjects => "オブジェクトファイル発見",
        Msg::FoundDuplicateGroups => "重複グループ発見",
        Msg::DuplicateFiles => "重複ファイル",
        Msg::Processing => "処理中...",

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
    }
}

fn msg_en(key: Msg) -> &'static str {
    match key {
        // Help
        Msg::AppDescription => "Share duplicate Git objects using hard links",
        Msg::ArgPath => "Target directory (default: current directory)",
        Msg::ArgDryRun => "Dry run (only show results without making changes)",
        Msg::ArgVerbose => "Verbose output",

        // Processing
        Msg::Scanning => "Scanning...",
        Msg::FoundObjects => "object files found",
        Msg::FoundDuplicateGroups => "duplicate groups found",
        Msg::DuplicateFiles => "duplicate files",
        Msg::Processing => "Processing...",

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
            Msg::ArgPath,
            Msg::ArgDryRun,
            Msg::ArgVerbose,
            Msg::Scanning,
            Msg::FoundObjects,
            Msg::FoundDuplicateGroups,
            Msg::DuplicateFiles,
            Msg::Processing,
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
        ];

        for key in keys {
            assert!(!msg_ja(key).is_empty());
            assert!(!msg_en(key).is_empty());
        }
    }
}
