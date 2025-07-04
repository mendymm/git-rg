use dashmap::DashSet;
use gix::ObjectId;
use gix::ThreadSafeRepository;
use gix::bstr::BStr;
use gix::bstr::ByteSlice;
use gix::bstr::ByteVec;
use gix::objs::tree;
use gix::revision::walk::Sorting::ByCommitTime;
use gix::traverse::commit::simple::CommitTimeOrder;
use gix::traverse::tree::visit::Action;
use gix::{Repository, bstr::BString};
use grep::{
    cli::{self, StandardStream},
    printer::{ColorSpecs, Standard, StandardBuilder},
    regex::RegexMatcher,
    searcher::{BinaryDetection, Searcher, SearcherBuilder},
};
use std::collections::VecDeque;
use std::io::IsTerminal as _;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use termcolor::ColorChoice;

pub static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn search_repo(thread_safe_repo: Arc<ThreadSafeRepository>, regex: &str) {
    let searched_objects = Arc::new(DashSet::<ObjectId>::new());
    let pool = rayon::ThreadPoolBuilder::default().build().unwrap();
    pool.scope(|s| {
        let repo = thread_safe_repo.to_thread_local();

        let current_commit = repo.head_commit().unwrap();
        search_commit(
            thread_safe_repo.clone(),
            current_commit.id,
            regex,
            searched_objects.clone(),
        );

        let ancestors = current_commit.ancestors();
        let walk = ancestors
            .sorting(ByCommitTime(CommitTimeOrder::NewestFirst))
            .all()
            .unwrap();

        for info in walk {
            let commit_id = info.unwrap().id;
            let thread_safe_repo = thread_safe_repo.clone();
            let searched_objects = searched_objects.clone();

            s.spawn(move |_s| {
                search_commit(thread_safe_repo, commit_id, regex, searched_objects);
            });
        }
    });
}

pub fn search_commit(
    thread_safe_repo: Arc<ThreadSafeRepository>,
    commit_id: ObjectId,
    regex: &str,
    searched_objects: Arc<DashSet<ObjectId>>,
) {
    let repo = thread_safe_repo.to_thread_local();
    let commit = repo.find_object(commit_id).unwrap().into_commit();
    // println!(
    //     "[{}] {}",
    //     &commit.id.to_hex_with_len(7),
    //     commit.time().unwrap().format(gix::date::time::format::GITOXIDE)
    // );

    let tree = commit.tree().unwrap();

    let platform = tree.traverse();

    let mut rec = GitSearcher::new(&repo, regex, searched_objects);
    platform.depthfirst(&mut rec).unwrap();
}

pub struct GitSearcher<'repo> {
    repo: &'repo Repository,
    searcher: Searcher,
    printer: Standard<StandardStream>,
    matcher: RegexMatcher,
    path: BString,
    path_deque: VecDeque<BString>,
    searched_objects: Arc<DashSet<ObjectId>>,
}

impl<'repo> GitSearcher<'repo> {
    pub fn new(
        repo: &'repo Repository,
        regex: &str,
        searched_objects: Arc<DashSet<ObjectId>>,
    ) -> Self {
        let matcher = RegexMatcher::new(regex).unwrap();
        let searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .line_number(true)
            .build();
        let printer = StandardBuilder::new()
            .color_specs(ColorSpecs::default_with_color())
            .build(cli::stdout(if std::io::stdout().is_terminal() {
                ColorChoice::Auto
            } else {
                ColorChoice::Never
            }));

        Self {
            repo,
            searcher,
            printer,
            matcher,
            path: Default::default(),
            path_deque: Default::default(),
            searched_objects,
        }
    }

    #[inline(always)]
    fn pop_element(&mut self) {
        if let Some(pos) = self.path.rfind_byte(b'/') {
            self.path.resize(pos, 0);
        } else {
            self.path.clear();
        }
    }

    #[inline(always)]
    fn push_element(&mut self, name: &BStr) {
        if name.is_empty() {
            return;
        }
        if !self.path.is_empty() {
            self.path.push(b'/');
        }
        self.path.push_str(name);
    }
}

impl<'repo> gix::traverse::tree::Visit for GitSearcher<'repo> {
    #[inline(always)]
    fn pop_back_tracked_path_and_set_current(&mut self) {
        self.path = self.path_deque.pop_back().unwrap_or_default();
    }

    #[inline(always)]
    fn pop_front_tracked_path_and_set_current(&mut self) {
        self.path = self
            .path_deque
            .pop_front()
            .expect("every call is matched with push_tracked_path_component");
    }

    #[inline(always)]
    fn push_back_tracked_path_component(&mut self, component: &BStr) {
        self.push_element(component);
        self.path_deque.push_back(self.path.clone());
    }

    #[inline(always)]
    fn push_path_component(&mut self, component: &BStr) {
        self.push_element(component);
    }

    #[inline(always)]
    fn pop_path_component(&mut self) {
        self.pop_element();
    }

    #[inline(always)]
    fn visit_tree(&mut self, _entry: &tree::EntryRef<'_>) -> Action {
        // self.records.push(Entry::new(entry, self.path_clone()));
        Action::Continue
    }

    #[inline(always)]
    fn visit_nontree(&mut self, entry: &tree::EntryRef<'_>) -> Action {
        if self.searched_objects.contains(entry.oid) {
            return Action::Skip;
        }
        let obj = self.repo.find_object(entry.oid).unwrap();

        let matcher_filepath = format!("[{}] {}", entry.oid.to_hex_with_len(7), self.path);

        self.searcher
            .search_slice(
                &self.matcher,
                &obj.data,
                self.printer
                    .sink_with_path(&self.matcher, &matcher_filepath),
            )
            .unwrap();
        self.searched_objects.insert(entry.oid.into());
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Action::Continue
    }
}
