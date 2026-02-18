use std::collections::HashSet;
use std::path::{Path, PathBuf};

use git_share_obj::cli::Args;
use git_share_obj::fsck::run_git_fsck;
use git_share_obj::hardlink::{replace_with_hardlink, ReplaceResult};
use git_share_obj::i18n::{format_size, msg, Msg};
use git_share_obj::scanner::{
    find_duplicates, find_git_repositories_with_progress, group_by_device, scan_git_objects_with_progress,
};

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

fn collect_repositories(paths: &[String], verbose: bool) -> Vec<PathBuf> {
    let mut repos = HashSet::new();
    for path_str in paths {
        let path = Path::new(path_str);
        if verbose {
            println!("{}: {}", msg(Msg::ScanningPath), path.display());
        }
        for repo in find_git_repositories_with_progress(path, |current| {
            if verbose {
                println!("{}: {}", msg(Msg::CheckingDirectory), current.display());
            }
        }) {
            repos.insert(repo);
        }
    }

    let mut repo_list: Vec<_> = repos.into_iter().collect();
    repo_list.sort();
    repo_list
}

fn run_fsck_checks(repos: &[PathBuf], verbose: bool) -> bool {
    let mut failed = 0usize;
    for repo in repos {
        if verbose {
            println!("{}: {}", msg(Msg::FsckRunning), repo.display());
        }

        let result = run_git_fsck(repo);
        if result.success {
            if verbose {
                println!("{}: {}", msg(Msg::FsckOk), repo.display());
            }
        } else {
            failed += 1;
            let detail = if result.stderr.is_empty() {
                format!("exit code: {:?}", result.code)
            } else {
                result.stderr
            };
            eprintln!("{}: {} - {}", msg(Msg::FsckFailed), repo.display(), detail);
        }
    }

    println!(
        "{}: {}/{} (failed: {})",
        msg(Msg::FsckSummary),
        repos.len().saturating_sub(failed),
        repos.len(),
        failed
    );
    failed == 0
}

fn main() {
    let args = Args::parse_args();

    // 全てのパスが存在するか確認
    for path_str in &args.paths {
        let path = Path::new(path_str);
        if !path.exists() {
            eprintln!("Error: {} does not exist", path_str);
            std::process::exit(1);
        }
    }

    let repos = collect_repositories(&args.paths, args.verbose);

    if args.fsck_only {
        let ok = run_fsck_checks(&repos, args.verbose);
        println!();
        println!("{}", msg(Msg::FsckOnlyComplete));
        if !ok {
            std::process::exit(2);
        }
        return;
    }

    if args.no_fsck {
        if args.verbose {
            println!("{}", msg(Msg::FsckSkipped));
        }
    } else {
        let ok = run_fsck_checks(&repos, args.verbose);
        if !ok {
            eprintln!("{}", msg(Msg::AbortOnFsckFailure));
            std::process::exit(2);
        }
    }

    if args.verbose {
        println!("{}", msg(Msg::Scanning));
    }

    // 全てのパスからオブジェクトファイルを収集
    let mut all_objects = Vec::new();
    for path_str in &args.paths {
        let path = Path::new(path_str);
        if args.verbose {
            println!("{}: {}", msg(Msg::ScanningPath), path.display());
        }
        let objects = scan_git_objects_with_progress(path, |current| {
            if args.verbose {
                println!("{}: {}", msg(Msg::CheckingDirectory), current.display());
            }
        });
        all_objects.extend(objects);
    }

    if args.verbose {
        println!("{}: {}", msg(Msg::FoundObjects), all_objects.len());
    }

    // デバイスIDでグループ化
    let device_groups = group_by_device(all_objects);
    let device_count = device_groups.len();

    if args.verbose && device_count > 1 {
        println!("{}: {}", msg(Msg::DeviceGroups), device_count);
    }

    let mut stats = Stats::new();

    // 各デバイスグループごとに処理
    for (device_id, objects) in device_groups {
        if args.verbose && device_count > 1 {
            println!("\n{}: {}", msg(Msg::ProcessingDevice), device_id);
        }

        // 重複を検出
        let duplicates = find_duplicates(objects);

        if args.verbose {
            println!("{}: {}", msg(Msg::FoundDuplicateGroups), duplicates.len());
        }

        // 重複がなければ次のデバイスグループへ
        if duplicates.is_empty() {
            if args.verbose {
                println!("{}: 0", msg(Msg::DuplicateFiles));
            }
            continue;
        }

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
                        ReplaceResult::RolledBack(e) => {
                            stats.errors += 1;
                            // ロールバック実行は常に出力
                            eprintln!("{}: {} - {}", msg(Msg::RollbackOccurred), dup.path.display(), e);
                        }
                        ReplaceResult::RollbackFailed(e) => {
                            stats.errors += 1;
                            // ロールバック失敗は常に出力
                            eprintln!("{}: {} - {}", msg(Msg::RollbackFailed), dup.path.display(), e);
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

    if !args.no_fsck && !args.dry_run {
        let ok = run_fsck_checks(&repos, args.verbose);
        if !ok {
            std::process::exit(3);
        }
    }
}
