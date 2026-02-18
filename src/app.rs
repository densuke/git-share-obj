use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::cli::Args;
use crate::fsck::run_git_fsck;
use crate::hardlink::{replace_with_hardlink, ReplaceResult};
use crate::i18n::{format_size, msg, Msg};
use crate::lock::{try_lock_repo, RepoLock};
use crate::scanner::{
    find_duplicates, find_git_repositories_with_progress, group_by_device, scan_git_objects_with_progress,
    GitObjectInfo,
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
        Self {
            total_duplicates: 0,
            replaced: 0,
            already_linked: 0,
            cross_filesystem: 0,
            errors: 0,
            total_savings: 0,
        }
    }
}

pub fn run(args: Args) -> i32 {
    if !validate_paths(&args.paths) {
        return 1;
    }

    let repos = collect_repositories(&args.paths, args.verbose);
    let (processing_repos, _locks) = if args.no_lock {
        if args.verbose {
            println!("{}", msg(Msg::LockSkipped));
        }
        (repos.clone(), Vec::new())
    } else {
        acquire_repo_locks(&repos, args.verbose)
    };

    if args.fsck_only {
        let ok = run_fsck_checks(&processing_repos, args.verbose);
        println!();
        println!("{}", msg(Msg::FsckOnlyComplete));
        return if ok { 0 } else { 2 };
    }

    if args.no_fsck {
        if args.verbose {
            println!("{}", msg(Msg::FsckSkipped));
        }
    } else if !run_fsck_checks(&processing_repos, args.verbose) {
        eprintln!("{}", msg(Msg::AbortOnFsckFailure));
        return 2;
    }

    if args.verbose {
        println!("{}", msg(Msg::Scanning));
    }

    let all_objects = collect_all_objects(&args.paths, args.verbose);
    if args.verbose {
        println!("{}: {}", msg(Msg::FoundObjects), all_objects.len());
    }

    let device_groups = group_by_device(all_objects);
    let device_count = device_groups.len();
    if args.verbose && device_count > 1 {
        println!("{}: {}", msg(Msg::DeviceGroups), device_count);
    }

    let mut stats = Stats::new();
    for (device_id, objects) in device_groups {
        if args.verbose && device_count > 1 {
            println!("\n{}: {}", msg(Msg::ProcessingDevice), device_id);
        }

        let duplicates = find_duplicates(objects);
        if args.verbose {
            println!("{}: {}", msg(Msg::FoundDuplicateGroups), duplicates.len());
        }

        if duplicates.is_empty() {
            if args.verbose {
                println!("{}: 0", msg(Msg::DuplicateFiles));
            }
            continue;
        }

        for group in &duplicates {
            let dup_count = group.duplicates.len();
            stats.total_duplicates += dup_count;
            let group_savings = group.source.size * dup_count as u64;
            stats.total_savings += group_savings;

            if args.dry_run {
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
                continue;
            }

            for dup in &group.duplicates {
                handle_replace_result(
                    replace_with_hardlink(&group.source.path, &dup.path),
                    dup.path.display().to_string(),
                    args.verbose,
                    &mut stats,
                );
            }
        }
    }

    print_summary(&args, &stats);

    if !args.no_fsck && !args.dry_run && !run_fsck_checks(&processing_repos, args.verbose) {
        return 3;
    }
    0
}

fn validate_paths(paths: &[String]) -> bool {
    for path_str in paths {
        let path = Path::new(path_str);
        if !path.exists() {
            eprintln!("Error: {} does not exist", path_str);
            return false;
        }
    }
    true
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

fn collect_all_objects(paths: &[String], verbose: bool) -> Vec<GitObjectInfo> {
    let mut all_objects = Vec::new();
    for path_str in paths {
        let path = Path::new(path_str);
        if verbose {
            println!("{}: {}", msg(Msg::ScanningPath), path.display());
        }
        let objects = scan_git_objects_with_progress(path, |current| {
            if verbose {
                println!("{}: {}", msg(Msg::CheckingDirectory), current.display());
            }
        });
        all_objects.extend(objects);
    }
    all_objects
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

fn acquire_repo_locks(repos: &[PathBuf], verbose: bool) -> (Vec<PathBuf>, Vec<RepoLock>) {
    let mut locked_repos = Vec::new();
    let mut locks = Vec::new();
    let mut failed = 0usize;

    for repo in repos {
        if verbose {
            println!("{}: {}", msg(Msg::LockingRepo), repo.display());
        }

        match try_lock_repo(repo) {
            Ok(lock) => {
                if verbose {
                    println!("{}: {}", msg(Msg::LockAcquired), repo.display());
                }
                locked_repos.push(repo.clone());
                locks.push(lock);
            }
            Err(e) => {
                failed += 1;
                eprintln!("{}: {} - {}", msg(Msg::LockFailed), repo.display(), e);
            }
        }
    }

    println!(
        "{}: {}/{} (failed: {})",
        msg(Msg::LockSummary),
        locked_repos.len(),
        repos.len(),
        failed
    );
    (locked_repos, locks)
}

fn handle_replace_result(result: ReplaceResult, path: String, verbose: bool, stats: &mut Stats) {
    match result {
        ReplaceResult::Replaced => {
            stats.replaced += 1;
            if verbose {
                println!("{}: {}", msg(Msg::Replaced), path);
            }
        }
        ReplaceResult::AlreadyLinked => {
            stats.already_linked += 1;
            if verbose {
                println!("{}: {}", msg(Msg::AlreadyLinked), path);
            }
        }
        ReplaceResult::CrossFilesystem => {
            stats.cross_filesystem += 1;
            println!("{}: {}", msg(Msg::CrossFilesystem), path);
        }
        ReplaceResult::RolledBack(e) => {
            stats.errors += 1;
            eprintln!("{}: {} - {}", msg(Msg::RollbackOccurred), path, e);
        }
        ReplaceResult::RollbackFailed(e) => {
            stats.errors += 1;
            eprintln!("{}: {} - {}", msg(Msg::RollbackFailed), path, e);
        }
        ReplaceResult::Error(e) => {
            stats.errors += 1;
            eprintln!("{}: {} - {}", msg(Msg::ErrorOccurred), path, e);
        }
    }
}

fn print_summary(args: &Args, stats: &Stats) {
    println!();
    if args.dry_run {
        println!("{}", msg(Msg::SummaryDryRun));
        println!("  {}: {}", msg(Msg::TotalDuplicates), stats.total_duplicates);
        println!("  {}: {}", msg(Msg::EstimatedSavings), format_size(stats.total_savings));
        return;
    }

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
