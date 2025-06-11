use std::{path::PathBuf, sync::Arc};

use dashmap::DashSet;
use gix::{ObjectId, ThreadSafeRepository};

fn main() {
    ensure_example_repo_is_initialized();
    divan::main();
}

fn ensure_example_repo_is_initialized(){
    let repo_path = PathBuf::from("example-repo");
    assert!(repo_path.exists());
}

#[divan::bench]
fn bench_find_in_commit(bencher: divan::Bencher) {
    bencher
        .with_inputs(|| {
            (
                ThreadSafeRepository::open("example-repo").unwrap(),
                ObjectId::from_hex(b"cbc598f245f3c157a872b69102653e2e349b6d92").unwrap(),
                r#"This string does not exist"#,
                Arc::new(DashSet::<ObjectId>::new()),
            )
        })
        .bench_values(|(repo,hash,regex,searched_objects)| {
            git_rg::search_commit(repo,hash,regex,searched_objects);
        });
}

#[divan::bench]
fn bench_find_in_repo(bencher: divan::Bencher) {
    bencher
        .with_inputs(|| {
            (
                ThreadSafeRepository::open("example-repo").unwrap(),
                r#"This string does not exist"#,
            )
        })
        .bench_values(|(repo,regex)| {
            git_rg::search_repo(repo,regex);
        });
}
