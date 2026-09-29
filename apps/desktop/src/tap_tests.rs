use super::*;

/// The command runs through `sh -c` exactly as tmux runs it, so the test is
/// what the shell makes of the quoting, not what the string looks like.
#[test]
fn a_pane_is_copied_into_a_fifo_whose_path_has_a_quote_in_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let odd = dir.path().join("it's here; $(no)");
    std::fs::create_dir(&odd).expect("dir");
    let copy = odd.join("leaf.fifo");

    let mut sh = std::process::Command::new("sh")
        .arg("-c")
        .arg(pipe_command(&copy))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("sh");
    {
        use std::io::Write;
        let mut stdin = sh.stdin.take().expect("stdin");
        stdin.write_all(b"heard").expect("write");
    }
    assert!(sh.wait().expect("wait").success());
    assert_eq!(std::fs::read_to_string(&copy).expect("copy"), "heard");
}

#[test]
fn a_tap_whose_window_went_is_let_go_and_no_other_projects_is() {
    let windows = vec!["leaf_alive".to_owned()];
    let taps = [
        ("leaf_alive", "prj_a"),
        ("leaf_exited", "prj_a"),
        ("leaf_elsewhere", "prj_b"),
    ];
    assert_eq!(
        gone(taps.into_iter(), "prj_a", &windows),
        vec!["leaf_exited".to_owned()]
    );
}
