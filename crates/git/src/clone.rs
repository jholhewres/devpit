//! `git clone`, into a folder this product owns.

use std::path::{Path, PathBuf};

use crate::GitError;

/// The folder name a remote URL should land in.
///
/// The last path segment without `.git`, which is what the person already
/// calls the project. Anything that is not a plain name is refused rather than
/// sanitised: a URL that produces `..` or an absolute path is not a typo to be
/// cleaned up, it is a path traversal, and this runs with the user's rights.
pub fn folder_for(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    let tail = trimmed.rsplit(['/', ':']).next()?;
    let name = tail.strip_suffix(".git").unwrap_or(tail);

    let plain = !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.starts_with('-');

    plain.then(|| name.to_owned())
}

/// Makes `dir` a repository of its own, if it is not one already.
///
/// Asked of `dir` itself, not of whatever repository above it would answer:
/// a folder inside another checkout would otherwise pass as initialised.
pub fn init(dir: &Path) -> Result<(), GitError> {
    if dir.join(".git").exists() {
        return Ok(());
    }
    crate::invoke::run(dir, &["init", "--initial-branch=main", "-q"]).map(|_| ())
}

/// Clones `url` into `parent/<name>` and returns where it landed.
///
/// The destination must not already exist: git would refuse anyway, and
/// failing here means the message names the folder rather than quoting git at
/// someone.
pub fn clone(url: &str, parent: &Path) -> Result<PathBuf, GitError> {
    let name = folder_for(url).ok_or_else(|| GitError::Unreadable {
        command: "clone".to_owned(),
        detail: format!("no folder name can be taken from {url}"),
    })?;

    std::fs::create_dir_all(parent).map_err(|err| GitError::Failed {
        command: "clone".to_owned(),
        stderr: err.to_string(),
    })?;

    let into = parent.join(&name);
    if into.exists() {
        return Err(GitError::Failed {
            command: "clone".to_owned(),
            stderr: format!("{} already exists", into.display()),
        });
    }

    let output = devpit_pty::host_env::command("git")
        // Never prompt. A clone that stops on a credential question with no
        // terminal attached hangs the command and the window with it; failing
        // with "authentication required" is a thing the screen can say.
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "")
        .arg("clone")
        .arg(url)
        .arg(&into)
        .output()
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => GitError::Missing,
            _ => GitError::Failed {
                command: "clone".to_owned(),
                stderr: err.to_string(),
            },
        })?;

    if !output.status.success() {
        // A half-written directory left behind would make the retry fail on
        // "already exists", which reads as the wrong problem.
        let _ = std::fs::remove_dir_all(&into);
        return Err(GitError::Failed {
            command: "clone".to_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    Ok(into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    #[test]
    fn a_folder_name_is_taken_from_either_url_shape() {
        assert_eq!(
            folder_for("https://github.com/o/repo.git").as_deref(),
            Some("repo")
        );
        assert_eq!(
            folder_for("git@github.com:o/repo.git").as_deref(),
            Some("repo")
        );
        assert_eq!(
            folder_for("https://github.com/o/repo/").as_deref(),
            Some("repo")
        );
    }

    /// The guard that matters: this name is joined onto a directory and then
    /// handed to git, which runs with the user's rights.
    ///
    /// `https://host/` is deliberately not in this list. It yields `host`,
    /// which is a plain folder name and not a traversal — the URL is useless
    /// and git refuses it, but nothing about it is dangerous, and asserting
    /// otherwise would be testing a claim this function does not make.
    #[test]
    fn a_url_that_would_climb_out_or_pass_a_flag_is_refused() {
        for hostile in [
            "https://host/o/..",
            "https://host/o/.",
            "ssh://host/-x",
            "https://host/o/a/b",
        ] {
            let name = folder_for(hostile);
            assert!(
                !matches!(name.as_deref(), Some("..") | Some(".") | Some("-x")),
                "{hostile} produced {name:?}"
            );
        }
        assert_eq!(folder_for("https://host/o/.."), None);
        assert_eq!(folder_for("ssh://host/-x"), None);
    }

    #[test]
    fn clones_a_local_repository_and_refuses_to_overwrite_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let origin = dir.path().join("origin");
        std::fs::create_dir_all(&origin).expect("create");
        fixture::repo(&origin);
        std::fs::write(origin.join("a.txt"), "one\n").expect("write");
        fixture::commit(&origin, "first");

        let into = dir.path().join("repos");
        let landed = clone(origin.to_str().expect("utf8"), &into).expect("clone");
        assert_eq!(landed, into.join("origin"));
        assert!(
            landed.join("a.txt").is_file(),
            "the working tree is missing"
        );

        let again = clone(origin.to_str().expect("utf8"), &into);
        assert!(again.is_err(), "the second clone overwrote the first");
    }
}
