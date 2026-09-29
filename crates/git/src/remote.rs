//! The remote: fetch, pull, push — and sync, which is pull then push.
//!
//! Pull only fast-forwards. A merge or a rebase started from a button can stop
//! half-way on a conflict, and a checkout left mid-merge is a thing to resolve
//! in a terminal, where the person can see it — so it is refused here and git's
//! own words say why.

use std::path::Path;

use crate::GitError;

/// Brings down what the remote has, and forgets branches it deleted.
pub fn fetch(root: &Path) -> Result<(), GitError> {
    talk(root, &["fetch", "--prune"])
}

/// Moves this branch forward to its upstream, when that is only forward.
pub fn pull(root: &Path) -> Result<(), GitError> {
    talk(root, &["pull", "--ff-only"])
}

/// Sends this branch up, giving it an upstream the first time.
pub fn push(root: &Path) -> Result<(), GitError> {
    if crate::invoke::run(root, &["rev-parse", "--abbrev-ref", "@{upstream}"]).is_ok() {
        return talk(root, &["push"]);
    }
    let remote = first_remote(root)?;
    talk(root, &["push", "--set-upstream", &remote, "HEAD"])
}

/// Pull, then push: what the branch's remote has, then what it does not.
pub fn sync(root: &Path) -> Result<(), GitError> {
    pull(root)?;
    push(root)
}

/// `origin` when there is one, else the only remote there is.
fn first_remote(root: &Path) -> Result<String, GitError> {
    let listed = crate::invoke::run(root, &["remote"])?;
    let names: Vec<&str> = listed
        .lines()
        .map(str::trim)
        .filter(|one| !one.is_empty())
        .collect();
    names
        .iter()
        .find(|one| **one == "origin")
        .or(names.first())
        .map(|one| (*one).to_owned())
        .ok_or_else(|| GitError::Refused("this checkout has no remote to push to".to_owned()))
}

/// git over the network, which must never ask: with no terminal attached a
/// credential prompt hangs the command, and the button with it.
fn talk(root: &Path, args: &[&str]) -> Result<(), GitError> {
    let output = devpit_pty::host_env::command("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "")
        .env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => GitError::Missing,
            _ => GitError::Failed {
                command: args.join(" "),
                stderr: err.to_string(),
            },
        })?;
    if output.status.success() {
        return Ok(());
    }
    Err(GitError::Failed {
        command: args.join(" "),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    fn git(at: &Path, args: &[&str]) {
        let ok = devpit_pty::host_env::command("git")
            .arg("-C")
            .arg(at)
            .args(args)
            .output()
            .expect("git")
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    }

    /// A bare remote, and two clones of it: `mine` and `theirs`.
    fn remote_and_two_clones(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let bare = dir.join("remote.git");
        std::fs::create_dir_all(&bare).expect("mkdir");
        git(&bare, &["init", "--bare", "--initial-branch=main", "-q"]);
        let mine = dir.join("mine");
        std::fs::create_dir_all(&mine).expect("mkdir");
        fixture::repo(&mine);
        std::fs::write(mine.join("a.txt"), "one\n").expect("write");
        fixture::commit(&mine, "first");
        git(
            &mine,
            &["remote", "add", "origin", bare.to_str().expect("utf-8")],
        );
        push(&mine).expect("the first push sets the upstream");
        let theirs = dir.join("theirs");
        git(
            dir,
            &[
                "clone",
                "-q",
                bare.to_str().expect("utf-8"),
                theirs.to_str().expect("utf-8"),
            ],
        );
        git(&theirs, &["config", "user.email", "t@example.invalid"]);
        git(&theirs, &["config", "user.name", "T"]);
        (mine, theirs)
    }

    #[test]
    fn sync_brings_down_theirs_and_sends_up_mine() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (mine, theirs) = remote_and_two_clones(dir.path());
        std::fs::write(theirs.join("b.txt"), "theirs\n").expect("write");
        fixture::commit(&theirs, "theirs");
        git(&theirs, &["push", "-q"]);

        sync(&mine).expect("fast-forward, then nothing to push");
        assert!(
            mine.join("b.txt").exists(),
            "their commit did not come down"
        );

        std::fs::write(mine.join("c.txt"), "mine\n").expect("write");
        fixture::commit(&mine, "mine");
        sync(&mine).expect("pushed");
        pull(&theirs).expect("pulled");
        assert!(theirs.join("c.txt").exists(), "my commit did not go up");
    }

    #[test]
    fn a_pull_that_would_merge_is_refused_and_leaves_the_checkout_clean() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (mine, theirs) = remote_and_two_clones(dir.path());
        std::fs::write(theirs.join("a.txt"), "theirs\n").expect("write");
        fixture::commit(&theirs, "theirs");
        git(&theirs, &["push", "-q"]);
        std::fs::write(mine.join("a.txt"), "mine\n").expect("write");
        fixture::commit(&mine, "mine");

        assert!(
            pull(&mine).is_err(),
            "diverged history was merged from a button"
        );
        assert!(
            !mine.join(".git/MERGE_HEAD").exists(),
            "the checkout was left mid-merge"
        );
    }

    #[test]
    fn a_checkout_with_no_remote_says_so() {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
        fixture::commit(dir.path(), "first");
        assert!(matches!(push(dir.path()), Err(GitError::Refused(_))));
    }
}
