use git_rg::search_commit;
use gix::ThreadSafeRepository;
use gix::revision::walk::Sorting::ByCommitTime;
use gix::traverse::commit::simple::CommitTimeOrder;
use std::sync::atomic::AtomicUsize;

static COUNTER: AtomicUsize = AtomicUsize::new(0);



fn main() {
    let pool = rayon::ThreadPoolBuilder::default().build().unwrap();
    let regex = "test";
    pool.scope(|s| {
        let repo_path = "/home/mendy/code/man-db";
        let thread_safe_repo: ThreadSafeRepository = ThreadSafeRepository::open(repo_path).unwrap();
        let repo = thread_safe_repo.to_thread_local();

        let current_commit = repo.head_commit().unwrap();
        search_commit(current_commit.id, thread_safe_repo.clone(), regex);

        let ancestors = current_commit.ancestors();
        let walk = ancestors
            .sorting(ByCommitTime(CommitTimeOrder::NewestFirst))
            .all()
            .unwrap();

        for info in walk {
            let commit_id = info.unwrap().id;
            let thread_safe_repo = thread_safe_repo.clone();

            s.spawn(move |_s| {
                search_commit(commit_id, thread_safe_repo, regex);
            });
        }
    });
    dbg!(COUNTER.fetch_add(0, std::sync::atomic::Ordering::Relaxed));
}
