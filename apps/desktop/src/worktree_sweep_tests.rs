use std::path::{Path, PathBuf};

use devpit_core::Store;

use super::*;

fn git(at: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(at)
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn commit(at: &Path, message: &str) {
    git(at, &["add", "-A"]);
    git(at, &["commit", "-q", "-m", message]);
}

/// A project on `main` that ignores `target`, a store that knows it, and
/// devpit's build caches folder, all in one tempdir.
struct World {
    dir: tempfile::TempDir,
    store: Store,
    root: PathBuf,
    cache: PathBuf,
    project: String,
}

impl World {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("project");
        std::fs::create_dir_all(&root).expect("mkdir");
        git(&root, &["init", "-q", "--initial-branch=main"]);
        git(&root, &["config", "user.email", "t@example.invalid"]);
        git(&root, &["config", "user.name", "T"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        std::fs::write(root.join(".gitignore"), "target\n").expect("write");
        std::fs::write(root.join("a.txt"), "one\n").expect("write");
        commit(&root, "first");

        let store = Store::open(&dir.path().join("state.db")).expect("open");
        let project = store.add_project(&root, None).expect("project");
        store.ensure_board(&project).expect("board");
        let cache = dir.path().join("cache").join("devpit-target");
        std::fs::create_dir_all(&cache).expect("mkdir");
        Self {
            dir,
            store,
            root,
            cache,
            project,
        }
    }

    /// A card on the branch an older devpit named, which it never recorded.
    fn card(&self, title: &str) -> (String, PathBuf, String) {
        self.card_named(title, "devpit/{slug}-{id}")
    }

    /// A card with a checkout of its own, one commit in, a build in `target`
    /// and a build cache of its own. A branch on the default pattern is
    /// recorded on the card, as a checkout does now.
    fn card_named(&self, title: &str, pattern: &str) -> (String, PathBuf, String) {
        let column = self.store.columns(&self.project).expect("columns")[0]
            .id
            .clone();
        let id = self
            .store
            .create_card(&self.project, &column, title, "")
            .expect("card");
        let at = self
            .dir
            .path()
            .join("worktrees")
            .join(&self.project)
            .join(&id);
        let number = self.store.card_numbers(&self.project).expect("numbers")[&id];
        let branch = devpit_git::branch_for(pattern, title, Some(number), &id);
        let made = devpit_git::create(&self.root, &at, &branch, "HEAD").expect("worktree");
        self.store
            .set_card_front(&id, made.path.to_str(), Some(&made.base_ref))
            .expect("front");
        if pattern.is_empty() {
            self.store.set_card_branch(&id, &branch).expect("branch");
        }
        std::fs::write(at.join(format!("{title}.txt")), "work\n").expect("write");
        commit(&at, title);
        std::fs::create_dir_all(at.join("target/debug")).expect("mkdir");
        std::fs::write(at.join("target/debug/big"), vec![0u8; 4096]).expect("write");
        let cache = self.cache.join(format!("wt-{id}"));
        std::fs::create_dir_all(&cache).expect("mkdir");
        std::fs::write(cache.join("big"), vec![0u8; 8192]).expect("write");
        (id, at, branch)
    }

    fn merge(&self, branch: &str) {
        git(
            &self.root,
            &["merge", "-q", "--no-ff", "-m", "merge", branch],
        );
    }

    fn sweep(&self, rules: &Rules, live: &dyn Fn(&str, &Path) -> bool) -> CleanupRun {
        let places = Places {
            cache: Some(self.cache.clone()),
        };
        sweep(&self.store, &places, rules, now(), live)
    }

    fn branch_exists(&self, branch: &str) -> bool {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args([
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("refs/heads/{branch}"),
            ])
            .output()
            .expect("git")
            .status
            .success()
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs() as i64
}

fn nobody(_: &str, _: &Path) -> bool {
    false
}

const DEFAULTS: Rules = Rules {
    delete_landed: true,
};

#[test]
fn a_landed_card_loses_its_worktree_its_caches_and_its_branch() {
    let world = World::new();
    let (id, at, branch) = world.card("landed");
    world.merge(&branch);

    let run = world.sweep(&DEFAULTS, &nobody);

    assert!(!at.exists(), "the checkout is still there");
    assert!(
        !world.cache.join(format!("wt-{id}")).exists(),
        "the cache is still there"
    );
    assert!(!world.branch_exists(&branch), "the branch is still there");
    assert_eq!(
        world
            .store
            .card(&id)
            .expect("card")
            .expect("row")
            .worktree_path,
        None
    );
    assert_eq!(run.removed.len(), 1);
    assert_eq!(run.removed[0].card_id, id);
    assert_eq!(run.removed[0].why, SweptWhy::Landed);
    assert!(
        run.freed_bytes >= (4096 + 8192) as f64,
        "freed {}",
        run.freed_bytes
    );
    assert_eq!(run.branches, 1);
    assert!(run.kept.is_empty());
}

#[test]
fn a_landed_card_with_uncommitted_work_is_kept_and_listed() {
    let world = World::new();
    let (id, at, branch) = world.card("dirty");
    world.merge(&branch);
    std::fs::write(at.join("unsaved.txt"), "not committed\n").expect("write");

    let run = world.sweep(&DEFAULTS, &nobody);

    assert!(at.join("unsaved.txt").exists());
    assert!(world.cache.join(format!("wt-{id}")).exists());
    assert!(world.branch_exists(&branch));
    assert!(run.removed.is_empty());
    assert_eq!(run.kept.len(), 1);
    assert_eq!(run.kept[0].why, SweptWhy::Uncommitted);
}

#[test]
fn a_landed_card_with_a_session_running_in_it_is_kept_and_listed() {
    let world = World::new();
    let (id, at, branch) = world.card("busy");
    world.merge(&branch);
    let running = |card: &str, path: &Path| card == id && path == at;

    let run = world.sweep(&DEFAULTS, &running);

    assert!(at.exists());
    assert!(world.cache.join(format!("wt-{id}")).exists());
    assert!(world.branch_exists(&branch));
    assert!(run.removed.is_empty());
    assert_eq!(run.kept.len(), 1);
    assert_eq!(run.kept[0].why, SweptWhy::Running);
}

#[test]
fn with_the_switch_off_a_landed_card_keeps_everything() {
    let world = World::new();
    let (id, at, branch) = world.card("off");
    world.merge(&branch);

    let run = world.sweep(
        &Rules {
            delete_landed: false,
        },
        &nobody,
    );

    assert!(at.join("target/debug/big").exists());
    assert!(world.cache.join(format!("wt-{id}")).exists());
    assert!(world.branch_exists(&branch));
    assert!(run.removed.is_empty() && run.kept.is_empty());
}

#[test]
fn a_landed_card_whose_checkout_was_removed_by_hand_loses_its_cache_and_branch() {
    let world = World::new();
    let (id, at, branch) = world.card("by-hand");
    world.merge(&branch);
    devpit_git::remove(&world.root, &at, false).expect("remove");
    world.store.forget_card_checkout(&id).expect("forget");

    let run = world.sweep(&DEFAULTS, &nobody);

    assert!(!world.cache.join(format!("wt-{id}")).exists());
    assert!(!world.branch_exists(&branch));
    assert_eq!(run.removed.len(), 1);
    assert!(run.freed_bytes >= 8192.0);
}

#[test]
fn a_landed_card_on_a_new_branch_name_loses_its_worktree_and_its_branch() {
    let world = World::new();
    let (_id, at, branch) = world.card_named("Fix the sweep", "");
    assert!(branch.starts_with("fix/"), "{branch}");
    world.merge(&branch);

    let run = world.sweep(&DEFAULTS, &nobody);

    assert!(!at.exists(), "the checkout is still there");
    assert!(!world.branch_exists(&branch), "the branch is still there");
    assert_eq!(run.branches, 1);
}

#[test]
fn old_and_new_branches_of_checkouts_removed_by_hand_are_both_found_and_removed() {
    let world = World::new();
    let (old, old_at, old_branch) = world.card("older");
    let (new, new_at, new_branch) = world.card_named("newer", "");
    assert!(old_branch.starts_with("devpit/") && new_branch.starts_with("feat/"));
    for (id, at, branch) in [(&old, &old_at, &old_branch), (&new, &new_at, &new_branch)] {
        world.merge(branch);
        devpit_git::remove(&world.root, at, false).expect("remove");
        world.store.forget_card_checkout(id).expect("forget");
    }

    let run = world.sweep(&DEFAULTS, &nobody);

    assert!(
        !world.branch_exists(&old_branch),
        "the older branch is still there"
    );
    assert!(
        !world.branch_exists(&new_branch),
        "the newer branch is still there"
    );
    assert_eq!(run.branches, 2);
}

#[test]
fn a_card_whose_work_has_not_landed_is_left_alone() {
    let world = World::new();
    let (id, at, branch) = world.card("open");

    let run = world.sweep(&DEFAULTS, &nobody);

    assert!(at.join("target/debug/big").exists());
    assert!(world.cache.join(format!("wt-{id}")).exists());
    assert!(world.branch_exists(&branch));
    assert!(run.removed.is_empty() && run.kept.is_empty());
}

#[test]
fn nothing_outside_devpits_worktrees_and_caches_is_touched() {
    let world = World::new();
    let outside = world.dir.path().join("outside");
    for one in ["target", "cache", "folder"] {
        std::fs::create_dir_all(outside.join(one)).expect("mkdir");
        std::fs::write(outside.join(one).join("precious"), "keep\n").expect("write");
    }

    // A landed card whose `target` and cache are links out of devpit's folders.
    let (linked, at, branch) = world.card("linked");
    std::fs::remove_dir_all(at.join("target")).expect("rm");
    link(&outside.join("target"), &at.join("target"));
    let cache = world.cache.join(format!("wt-{linked}"));
    std::fs::remove_dir_all(&cache).expect("rm");
    link(&outside.join("cache"), &cache);
    world.merge(&branch);

    // A landed card whose recorded checkout is the project itself, and one
    // whose recorded checkout is a folder git does not know.
    let (main, main_at, main_branch) = world.card("main");
    world.merge(&main_branch);
    let began = |id: &str| world.store.card(id).expect("card").expect("row").base_ref;
    world
        .store
        .set_card_front(&main, world.root.to_str(), began(&main).as_deref())
        .expect("front");
    let (stray, _, stray_branch) = world.card("stray");
    world.merge(&stray_branch);
    world
        .store
        .set_card_front(
            &stray,
            outside.join("folder").to_str(),
            began(&stray).as_deref(),
        )
        .expect("front");

    // Caches nobody's card names.
    std::fs::create_dir_all(world.cache.join("wt-card_SOMEONEELSE")).expect("mkdir");
    std::fs::create_dir_all(world.cache.join("base-card-1")).expect("mkdir");

    world.sweep(&DEFAULTS, &nobody);

    for one in ["target", "cache", "folder"] {
        assert!(
            outside.join(one).join("precious").exists(),
            "{one} was touched"
        );
    }
    assert!(world.root.join("a.txt").exists(), "the project was touched");
    assert!(world.root.join(".git").exists());
    assert!(
        main_at.exists(),
        "a checkout the card no longer names was removed"
    );
    assert!(world.cache.join("wt-card_SOMEONEELSE").exists());
    assert!(world.cache.join("base-card-1").exists());
    assert!(!at.exists(), "the linked card's own checkout stayed");
}

#[cfg(unix)]
fn link(target: &Path, at: &Path) {
    std::os::unix::fs::symlink(target, at).expect("symlink");
}

#[cfg(windows)]
fn link(target: &Path, at: &Path) {
    std::os::windows::fs::symlink_dir(target, at).expect("symlink");
}

#[test]
fn the_defaults_delete_landed_worktrees_and_nothing_else() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");

    assert_eq!(
        rules_of(&store),
        Rules {
            delete_landed: true
        }
    );
}

#[test]
fn the_log_says_what_went_what_stayed_and_what_was_freed() {
    let swept = |why| Swept {
        card_id: "card_1".to_owned(),
        title: "One".to_owned(),
        why,
        bytes: 0.0,
        detail: None,
    };
    let run = CleanupRun {
        at: 0.0,
        removed: vec![swept(SweptWhy::Landed), swept(SweptWhy::Landed)],
        kept: vec![swept(SweptWhy::Running)],
        branches: 2,
        freed_bytes: 1.5 * 1024.0 * 1024.0 * 1024.0,
    };

    assert_eq!(
        said(&run),
        "worktree cleanup: removed 2 merged worktree(s) and 2 branch(es), kept 1; freed 1.5 GB"
    );
}
