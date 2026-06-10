use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore]
fn acceptance_delivery() {
    let repo = repo_root();
    for args in [
        &["check"][..],
        &["publish", "delivery"],
        &["apply"],
        &["status"],
        &["health"],
        &["client"],
    ] {
        run(&repo, "yaoe", args);
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root")
        .to_path_buf()
}

fn run(repo: &PathBuf, program: &str, args: &[&str]) {
    let status = Command::new(program)
        .current_dir(repo)
        .args(args)
        .status()
        .unwrap_or_else(|err| panic!("run {program}: {err}"));
    assert!(status.success(), "{program} {args:?} failed");
}
