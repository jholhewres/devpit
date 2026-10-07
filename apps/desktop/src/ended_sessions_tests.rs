use super::*;

const ID: &str = "0c1b7a2e-5d4f-4a3b-9c8d-7e6f5a4b3c2d";

#[test]
fn an_ended_session_resumes_as_the_same_conversation_under_its_name() {
    let line = resumed(
        "claude --settings /s.json",
        "api-a",
        Some("--resume {}"),
        ID,
    )
    .expect("line");
    assert_eq!(
        line,
        format!("claude --settings /s.json --name 'api-a' --resume {ID}")
    );
}

#[test]
fn a_resume_that_would_start_afresh_or_run_something_else_is_refused() {
    assert!(
        resumed("claude", "api-a", None, ID).is_err(),
        "an agent with no resume started afresh"
    );
    assert!(resumed("claude", "api-a", Some("--resume {}"), "abc; rm -rf ~").is_err());
    assert!(resumed("claude", "a\x03b", Some("--resume {}"), ID).is_err());
}

#[test]
fn files_left_uncommitted_in_a_folder_are_counted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .output()
            .expect("git");
    };
    git(&["init", "-q"]);
    assert_eq!(dirty_in(&dir.path().display().to_string()), Some(0));
    std::fs::write(dir.path().join("half.rs"), "fn").expect("write");
    assert_eq!(dirty_in(&dir.path().display().to_string()), Some(1));
    assert_eq!(dirty_in("/no/such/folder"), None);
}

#[test]
fn a_stop_that_would_lose_work_says_why_and_asks() {
    use crate::stopping::would_lose;
    assert_eq!(would_lose("a7", false, Some(0)), None);
    assert_eq!(would_lose("a7", false, None), None);
    assert_eq!(
        would_lose("a7", true, None).as_deref(),
        Some("a7: it is working right now — stop it anyway?")
    );
    assert_eq!(
        would_lose("a7", false, Some(12)).as_deref(),
        Some("a7: its folder has 12 file(s) not committed — stop it anyway?")
    );
}
