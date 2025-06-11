use gix::ObjectId;
use gix::ThreadSafeRepository;
use gix::bstr::BStr;
use gix::bstr::ByteSlice;
use gix::bstr::ByteVec;
use gix::objs::tree;
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
use std::sync::atomic::AtomicUsize;
use termcolor::ColorChoice;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// #[cfg(not(target_env = "msvc"))]
// use tikv_jemallocator::Jemalloc;

// #[cfg(not(target_env = "msvc"))]
// #[global_allocator]
// static GLOBAL: Jemalloc = Jemalloc;

pub fn search_commit(commit_id: ObjectId, thread_safe_repo: ThreadSafeRepository, regex: &str) {
    let repo = thread_safe_repo.to_thread_local();
    let commit = repo.find_object(commit_id).unwrap().into_commit();
    // println!(
    //     "[{}] {}",
    //     &commit.id.to_hex_with_len(7),
    //     commit.time().unwrap().format(format::GITOXIDE)
    // );

    let tree = commit.tree().unwrap();

    let platform = tree.traverse();

    let mut rec = GitSearcher::new(&repo, regex);
    platform.depthfirst(&mut rec).unwrap();
}
pub struct GitSearcher<'repo> {
    repo: &'repo Repository,
    searcher: Searcher,
    printer: Standard<StandardStream>,
    matcher: RegexMatcher,
    path: BString,
    path_deque: VecDeque<BString>,
}

impl<'repo> GitSearcher<'repo> {
    pub fn new(repo: &'repo Repository, regex: &str) -> Self {
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
        }
    }

    fn pop_element(&mut self) {
        if let Some(pos) = self.path.rfind_byte(b'/') {
            self.path.resize(pos, 0);
        } else {
            self.path.clear();
        }
    }

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
    fn pop_back_tracked_path_and_set_current(&mut self) {
        self.path = self.path_deque.pop_back().unwrap_or_default();
    }

    fn pop_front_tracked_path_and_set_current(&mut self) {
        self.path = self
            .path_deque
            .pop_front()
            .expect("every call is matched with push_tracked_path_component");
    }

    fn push_back_tracked_path_component(&mut self, component: &BStr) {
        self.push_element(component);
        self.path_deque.push_back(self.path.clone());
    }

    fn push_path_component(&mut self, component: &BStr) {
        self.push_element(component);
    }

    fn pop_path_component(&mut self) {
        self.pop_element();
    }

    fn visit_tree(&mut self, _entry: &tree::EntryRef<'_>) -> Action {
        // self.records.push(Entry::new(entry, self.path_clone()));
        Action::Continue
    }

    fn visit_nontree(&mut self, entry: &tree::EntryRef<'_>) -> Action {
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
        Action::Continue
    }
}

// pub struct MatchString{

// }

// impl grep::matcher::Matcher for MatchString {
//     type Captures;

//     type Error;

//     fn find_at(
//         &self,
//         haystack: &[u8],
//         at: usize,
//     ) -> Result<Option<grep::matcher::Match>, Self::Error> {
//         todo!()
//     }

//     fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
//         todo!()
//     }
// }
