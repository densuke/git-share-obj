use std::path::Path;

use git_share_obj::cli::Args;
use git_share_obj::hardlink::{replace_with_hardlink, ReplaceResult};
use git_share_obj::i18n::{format_size, msg, Msg};
use git_share_obj::scanner::{find_duplicates, scan_git_objects};

/// 処理統計
struct Stats {
    total_duplicates: usize,
    replaced: usize,
    already_linked: usize,
    cross_filesystem: usize,
    errors: usize,
    total_savings: u64,
}

impl Stats {
    fn new() -> Self {
        Stats {
            total_duplicates: 0,
            replaced: 0,
            already_linked: 0,
            cross_filesystem: 0,
            errors: 0,
            total_savings: 0,
        }
    }
}

fn main() {
    let args = Args::parse_args();
    let path = Path::new(&args.path);

    if !path.exists() {
        eprintln!("Error: {} does not exist", args.path);
        std::process::exit(1);
    }

    if args.verbose {
        println!("{}", msg(Msg::Scanning));
    }

    // オブジェクトファイルを探索
    let objects = scan_git_objects(path);

    if args.verbose {
        println!("{}: {}", msg(Msg::FoundObjects), objects.len());
    }

    // 重複を検出
    let duplicates = find_duplicates(objects);

    if args.verbose {
        println!("{}: {}", msg(Msg::FoundDuplicateGroups), duplicates.len());
    }

    // 重複がなければ終了
    if duplicates.is_empty() {
        if args.verbose {
            println!("{}: 0", msg(Msg::DuplicateFiles));
        }
        return;
    }

    let mut stats = Stats::new();

    // 各重複グループを処理
    for group in &duplicates {
        let dup_count = group.duplicates.len();
        stats.total_duplicates += dup_count;

        // グループの削減容量を計算 (重複ファイル数 × ファイルサイズ)
        let group_savings = group.source.size * dup_count as u64;
        stats.total_savings += group_savings;

        if args.dry_run {
            // ドライランモード: 検出結果と削減容量を表示
            if args.verbose {
                println!(
                    "\n{}: {} ({}: {})",
                    msg(Msg::DuplicateFiles),
                    dup_count + 1,
                    msg(Msg::GroupSavings),
                    format_size(group_savings)
                );
                println!("  [source] {} ({})", group.source.path.display(), format_size(group.source.size));
                for dup in &group.duplicates {
                    println!("  [dup]    {}", dup.path.display());
                }
            }
        } else {
            // 実行モード: ハードリンクに置換
            for dup in &group.duplicates {
                let result = replace_with_hardlink(&group.source.path, &dup.path);

                match &result {
                    ReplaceResult::Replaced => {
                        stats.replaced += 1;
                        if args.verbose {
                            println!("{}: {}", msg(Msg::Replaced), dup.path.display());
                        }
                    }
                    ReplaceResult::AlreadyLinked => {
                        stats.already_linked += 1;
                        if args.verbose {
                            println!("{}: {}", msg(Msg::AlreadyLinked), dup.path.display());
                        }
                    }
                    ReplaceResult::CrossFilesystem => {
                        stats.cross_filesystem += 1;
                        // FS跨ぎは通常モードでも出力
                        println!("{}: {}", msg(Msg::CrossFilesystem), dup.path.display());
                    }
                    ReplaceResult::Error(e) => {
                        stats.errors += 1;
                        // エラーは常に出力
                        eprintln!("{}: {} - {}", msg(Msg::ErrorOccurred), dup.path.display(), e);
                    }
                }
            }
        }
    }

    // サマリーを出力
    println!();
    if args.dry_run {
        println!("{}", msg(Msg::SummaryDryRun));
        println!("  {}: {}", msg(Msg::TotalDuplicates), stats.total_duplicates);
        println!("  {}: {}", msg(Msg::EstimatedSavings), format_size(stats.total_savings));
    } else {
        println!("{}", msg(Msg::SummaryComplete));
        println!("  {}: {}", msg(Msg::TotalDuplicates), stats.total_duplicates);
        println!("  {}: {}", msg(Msg::TotalReplaced), stats.replaced);
        let skipped = stats.already_linked + stats.cross_filesystem;
        println!("  {}: {}", msg(Msg::TotalSkipped), skipped);
        if stats.errors > 0 {
            println!("  {}: {}", msg(Msg::TotalErrors), stats.errors);
        }
        println!("  {}: {}", msg(Msg::TotalSavings), format_size(stats.total_savings));
    }
}
