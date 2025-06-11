use std::sync::Arc;

use git_rg::search_repo;
use gix::ThreadSafeRepository;

fn main() {
    let regex = "gimme gimme gimme";
    let repo_path = "/home/mendy/code/man-db";

    search_repo(
        Arc::new(ThreadSafeRepository::open(repo_path).unwrap()),
        regex,
    );

    let total_count = git_rg::COUNTER.fetch_add(0, std::sync::atomic::Ordering::Relaxed);
    dbg!(total_count);
}
