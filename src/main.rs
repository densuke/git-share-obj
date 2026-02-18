use git_share_obj::cli::Args;

fn main() {
    let args = Args::parse_args();

    if args.verbose {
        println!("対象ディレクトリ: {}", args.path);
        println!("ドライラン: {}", args.dry_run);
    }
}
