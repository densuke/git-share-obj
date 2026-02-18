use std::process;

use git_share_obj::app::run;
use git_share_obj::cli::Args;

fn main() {
    let code = run(Args::parse_args());
    if code != 0 {
        process::exit(code);
    }
}
