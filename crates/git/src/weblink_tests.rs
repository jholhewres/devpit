use super::*;
use crate::fixture;

const SHA: &str = "579b1b2aa";

#[test]
fn every_way_of_writing_a_github_remote_is_the_same_page() {
    let page = Some(format!("https://github.com/o/r/commit/{SHA}"));
    for remote in [
        "git@github.com:o/r.git",
        "git@github.com:o/r",
        "https://github.com/o/r.git",
        "https://github.com/o/r/",
        "https://someone@github.com/o/r.git",
        "ssh://git@github.com/o/r.git",
        "ssh://git@github.com:22/o/r.git",
    ] {
        assert_eq!(web_url(remote, SHA), page, "{remote}");
    }
}

#[test]
fn each_forge_gets_its_own_path_to_a_commit() {
    assert_eq!(
        web_url("git@bitbucket.org:team/repo.git", SHA).as_deref(),
        Some("https://bitbucket.org/team/repo/commits/579b1b2aa")
    );
    assert_eq!(
        web_url("https://gitlab.com/group/sub/repo.git", SHA).as_deref(),
        Some("https://gitlab.com/group/sub/repo/-/commit/579b1b2aa")
    );
}

#[test]
fn a_remote_on_disk_has_no_page() {
    assert_eq!(web_url("/srv/git/repo.git", SHA), None);
    assert_eq!(web_url("../other", SHA), None);
    assert_eq!(web_url("", SHA), None);
}

#[test]
fn a_short_id_is_given_back_whole_with_no_remote() {
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "a").expect("write");
    fixture::commit(dir.path(), "first");
    let short = run(dir.path(), &["rev-parse", "--short", "HEAD"]).expect("short");
    let link = commit_link(dir.path(), short.trim()).expect("link");
    assert_eq!(link.full.len(), 40);
    assert_eq!(link.url, None);

    run(
        dir.path(),
        &["remote", "add", "origin", "git@github.com:o/r.git"],
    )
    .expect("remote");
    let link = commit_link(dir.path(), short.trim()).expect("link");
    assert_eq!(
        link.url,
        Some(format!("https://github.com/o/r/commit/{}", link.full))
    );
}
