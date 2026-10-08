use super::{keep, listed, plain_name, with_secret};

#[test]
fn a_secret_replaces_its_own_line_and_leaves_the_rest() {
    let env = "# app\nexport JEV_API_KEY=old\nOTHER=1\n";
    assert_eq!(
        with_secret(env, "JEV_API_KEY", "new"),
        "# app\nJEV_API_KEY=new\nOTHER=1\n"
    );
    // A name that only starts like it is another variable.
    assert_eq!(
        with_secret("JEV_API_KEY_2=x\n", "JEV_API_KEY", "y"),
        "JEV_API_KEY_2=x\nJEV_API_KEY=y\n"
    );
    assert_eq!(with_secret("", "A", "x y\"z"), "A=\"x y\\\"z\"\n");
}

#[test]
fn only_a_plain_name_is_taken() {
    assert!(plain_name("JEV_API_KEY") && plain_name("_X1"));
    for bad in ["", "jev", "A-B", "1A", "A B", "A=1"] {
        assert!(!plain_name(bad), "{bad}");
    }
}

#[cfg(unix)]
#[test]
fn a_kept_secret_is_private_out_of_git_and_its_value_is_not_listed() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("create");
    assert!(std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .status()
        .expect("git")
        .success());
    let kept = dir.path().join("home").join("secrets.json");

    let list = keep(&root, &kept, "HF_TOKEN", "hf_value_that_must_stay_put").expect("keep");
    let env = root.join(".env");
    assert_eq!(
        std::fs::read_to_string(&env).expect("env"),
        "HF_TOKEN=hf_value_that_must_stay_put\n"
    );
    assert_eq!(
        std::fs::metadata(&env).expect("meta").permissions().mode() & 0o777,
        0o600
    );
    assert!(devpit_git::ignored(&root, ".env").expect("ignored"));
    assert!(!root.join(".gitignore").exists());
    assert_eq!(list.secrets.len(), 1);
    assert_eq!(listed(&kept)[0].name, "HF_TOKEN");
    assert!(!std::fs::read_to_string(&kept)
        .expect("kept")
        .contains("hf_value"));
}

#[test]
fn a_bad_value_is_refused_without_repeating_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let err = keep(
        dir.path(),
        &dir.path().join("s.json"),
        "KEY",
        "two\nlines-sk-secret",
    )
    .expect_err("refused");
    assert!(!err.message.contains("sk-secret"));
}
