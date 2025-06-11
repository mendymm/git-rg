use git_rg::search_repo;
use gix::ThreadSafeRepository;

fn main() {
    let regex = "test";
    let repo_path = "/home/mendy/code/man-db";

    search_repo(ThreadSafeRepository::open(repo_path).unwrap(), regex);
}
