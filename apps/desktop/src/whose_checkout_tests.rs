use super::*;
use serde_json::json;

fn project(id: &str, root: &Path) -> Project {
    serde_json::from_value(json!({
        "id": id,
        "name": id,
        "rootPath": root.display().to_string(),
        "group": null,
        "accent": "#000000",
        "worktrees": [],
        "unreadable": null,
        "origin": null,
        "lastOpenedAt": null,
        "icon": null,
        "color": null,
    }))
    .expect("a project")
}

fn git(dir: &Path, args: &[&str]) {
    let done = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .expect("git");
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
}

/// A repository with one commit and a worktree outside it, as devpit makes
/// a card's checkout.
fn repo_with_worktree() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let main = dir.path().join("app");
    std::fs::create_dir_all(main.join("src")).expect("mkdir");
    git(&main, &["init", "-q", "-b", "main"]);
    git(&main, &["commit", "-q", "--allow-empty", "-m", "one"]);
    let tree = dir.path().join("worktrees").join("card-1");
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "card-1",
            tree.to_str().expect("utf-8"),
        ],
    );
    (dir, main, tree)
}

#[test]
fn a_worktree_leads_to_its_main_checkout_without_asking_git() {
    let (_dir, main, tree) = repo_with_worktree();
    std::fs::create_dir_all(tree.join("deep")).expect("mkdir");
    assert_eq!(
        main_root(&tree.join("deep")).map(|p| real(&p)),
        Some(real(&main))
    );
    assert_eq!(
        main_root(&main.join("src")).map(|p| real(&p)),
        Some(real(&main))
    );
}

#[test]
fn the_owner_of_a_worktree_is_the_project_of_its_repository() {
    let (dir, main, tree) = repo_with_worktree();
    // Another project whose folder holds the worktree: the repository wins.
    let projects = vec![project("outer", dir.path()), project("app", &main)];
    assert_eq!(owner(&projects, &tree).map(|p| p.id.as_str()), Some("app"));
    assert_eq!(
        owner(&projects, &main.join("src")).map(|p| p.id.as_str()),
        Some("app")
    );
}

#[test]
fn a_project_inside_a_repository_is_found_by_its_folder() {
    let (_dir, main, _tree) = repo_with_worktree();
    let sub = main.join("src");
    let projects = vec![project("sub", &sub)];
    assert_eq!(owner(&projects, &sub).map(|p| p.id.as_str()), Some("sub"));
    assert!(owner(&projects, &main).is_none());
}
