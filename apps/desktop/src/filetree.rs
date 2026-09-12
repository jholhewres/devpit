//! One level of the file tree, with git's opinion of each entry folded in.
//!
//! Apart from `projects.rs` because it is a different question: that file owns
//! the project row, and this one owns what the tree looks like from a given
//! checkout — including the rule that a folder shows the loudest change under
//! it, which is the only real decision in here.

use devpit_core::tree;
use devpit_rpc::{FileNode, GitStatus, ProjectTree, RpcError};

use crate::projects::{checkout, locate, store};

fn tree_error(err: devpit_core::TreeError) -> RpcError {
    match err {
        // Its own code, not a generic one: the screen says something different
        // for a path that escaped than for a folder it could not read, and
        // this process runs terminals — reaching it is reaching the machine.
        devpit_core::TreeError::Outside { .. } => RpcError::forbidden(err.to_string()),
        other => RpcError::internal(other.to_string()),
    }
}

/// `project.tree` — one level of the file tree, from a given worktree.
///
/// One level rather than the whole tree: a monorepo has hundreds of thousands
/// of files and the screen draws only what is expanded. `path` is empty for
/// the root.
#[tauri::command]
#[specta::specta]
pub fn project_tree(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<ProjectTree, RpcError> {
    let store = store()?;
    let (_, root) = locate(&store, &project_id)?;
    let root = checkout(&root, worktree_id.as_deref());

    let status = devpit_git::status(&root)
        .map(|status| status.paths)
        .unwrap_or_default();

    let nodes = tree::children(&root, &path)
        .map_err(tree_error)?
        .into_iter()
        .map(|entry| FileNode {
            status: mark(&entry, &status),
            children: entry.is_dir.then(Vec::new),
            name: entry.name,
            path: entry.path,
        })
        .collect();

    Ok(ProjectTree { nodes })
}

/// How much a status wants to be seen, when several are collapsed into one row.
///
/// A deletion outranks a modification outranks an addition: the further left,
/// the harder it is to undo by accident. Untracked is last because it is the
/// normal state of a working tree, not news.
fn loudness(status: GitStatus) -> u8 {
    match status {
        GitStatus::Deleted => 4,
        GitStatus::Modified => 3,
        GitStatus::Added => 2,
        GitStatus::Untracked => 1,
        GitStatus::Clean => 0,
    }
}

/// A directory shows the loudest thing under it.
///
/// Otherwise a change three levels down is invisible until you have opened
/// three folders looking for it, which is the opposite of what the colour is
/// for. Ranked rather than first-found: the map is ordered by path, and taking
/// its first entry would show whichever file happens to sort earliest.
fn mark(entry: &tree::Entry, status: &std::collections::BTreeMap<String, GitStatus>) -> GitStatus {
    if !entry.is_dir {
        return status.get(&entry.path).copied().unwrap_or(GitStatus::Clean);
    }

    let prefix = format!("{}/", entry.path);
    status
        .iter()
        .filter(|(path, _)| path.starts_with(&prefix))
        .map(|(_, status)| *status)
        .max_by_key(|status| loudness(*status))
        .unwrap_or(GitStatus::Clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(path: &str) -> tree::Entry {
        tree::Entry {
            name: path.rsplit('/').next().unwrap_or(path).to_owned(),
            path: path.to_owned(),
            is_dir: true,
        }
    }

    #[test]
    fn a_directory_shows_its_loudest_change_not_its_first() {
        // `a.rs` sorts before `z.rs`, so first-found would report Modified and
        // the deletion under the same folder would go unseen.
        let status = std::collections::BTreeMap::from([
            ("web/a.rs".to_owned(), GitStatus::Modified),
            ("web/z.rs".to_owned(), GitStatus::Deleted),
        ]);
        assert_eq!(mark(&dir("web"), &status), GitStatus::Deleted);
    }

    #[test]
    fn a_directory_with_nothing_under_it_is_clean() {
        let status = std::collections::BTreeMap::from([
            // Same prefix, different folder: `webbing` must not count as `web`.
            ("webbing/a.rs".to_owned(), GitStatus::Modified),
        ]);
        assert_eq!(mark(&dir("web"), &status), GitStatus::Clean);
    }
}
