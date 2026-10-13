//! Which project an agent's folder belongs to, read from the files git keeps
//! rather than by asking git.
//!
//! Every agent question starts here, and asking each project's git for its
//! worktrees cost a process per project — seconds, under load, of the few the
//! agent waits. A checkout says where its repository is: `.git` is the
//! repository itself in the main checkout, and a file naming it in a worktree.

use std::path::{Path, PathBuf};

use devpit_rpc::Project;

/// The project whose repository holds `cwd`, or whose folder holds it; `None`
/// when neither says, and the caller asks git.
pub(crate) fn owner<'a>(projects: &'a [Project], cwd: &Path) -> Option<&'a Project> {
    if let Some(root) = main_root(cwd) {
        let root = real(&root);
        if let Some(project) = projects
            .iter()
            .find(|project| real(Path::new(&project.root_path)) == root)
        {
            return Some(project);
        }
    }
    // A project registered at a folder inside a repository, or with no git.
    crate::agent_api::project_at(projects, cwd)
}

/// The main checkout of the repository `cwd` is in: the folder holding
/// `.git` itself, reached through a worktree's `gitdir` and `commondir`.
pub(crate) fn main_root(cwd: &Path) -> Option<PathBuf> {
    let checkout = cwd.ancestors().find(|dir| dir.join(".git").exists())?;
    let dot_git = checkout.join(".git");
    if dot_git.is_dir() {
        return Some(checkout.to_path_buf());
    }
    let text = std::fs::read_to_string(&dot_git).ok()?;
    let gitdir = text.lines().find_map(|line| line.strip_prefix("gitdir:"))?;
    let gitdir = checkout.join(gitdir.trim());
    match std::fs::read_to_string(gitdir.join("commondir")) {
        // A worktree: its repository's `.git`, whose folder is the main checkout.
        Ok(common) => {
            let common = real(&gitdir.join(common.trim()));
            (common.file_name()? == ".git").then(|| common.parent().map(Path::to_path_buf))?
        }
        // A submodule's or a separate gitdir: the checkout is its own root.
        Err(_) => Some(checkout.to_path_buf()),
    }
}

fn real(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
#[path = "whose_checkout_tests.rs"]
mod tests;
