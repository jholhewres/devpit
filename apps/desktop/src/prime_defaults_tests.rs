use super::declared_or_env;
use crate::prime::{run, Prime, Primed};

fn nothing() -> impl FnMut(&str) {
    |_| {}
}

#[test]
fn a_project_that_declared_nothing_gets_its_env_files_copied() {
    let dir = tempfile::tempdir().expect("tempdir");
    let main = dir.path().join("main");
    let side = dir.path().join("side");
    std::fs::create_dir_all(&main).expect("create");
    std::fs::create_dir_all(&side).expect("create");
    std::fs::write(main.join(".env"), "KEY=1\n").expect("write");
    std::fs::write(main.join(".env.local"), "LOCAL=1\n").expect("write");
    // Tracked, and so already in the worktree: left as it is there.
    std::fs::write(main.join(".env.example"), "KEY=\n").expect("write");
    std::fs::write(side.join(".env.example"), "KEY=worktree\n").expect("write");

    let declared = declared_or_env(&dir.path().join("prime.json"));
    assert_eq!(
        run(&declared, &main, &side, nothing()).expect("prime"),
        Primed::Done
    );
    assert_eq!(
        std::fs::read_to_string(side.join(".env")).expect("env"),
        "KEY=1\n"
    );
    assert!(side.join(".env.local").exists());
    assert_eq!(
        std::fs::read_to_string(side.join(".env.example")).expect("example"),
        "KEY=worktree\n"
    );
}

#[test]
fn a_declared_file_is_taken_as_it_is_even_when_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("prime.json");
    std::fs::write(&file, "{}").expect("write");
    assert_eq!(declared_or_env(&file), Prime::default());
}
