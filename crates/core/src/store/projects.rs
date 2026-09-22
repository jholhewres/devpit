//! Reading and writing the project rows.
//!
//! A project is a row here plus a repository on disk; this module owns the
//! row, and `crates/git` owns the repository. Neither reaches into the other.

use std::path::Path;

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

/// A project row, before git fills in what it knows.
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub group: Option<String>,
    pub accent: String,
    pub origin: Option<String>,
    pub last_opened_at: Option<i64>,
    /// The icon a person chose, when they chose one.
    pub icon: Option<String>,
    /// And the colour, as `#rrggbb`.
    pub color: Option<String>,
}

/// The default trust workspace, created on first use.
///
/// `project.trust_workspace_id` is NOT NULL, so a project cannot exist without
/// one. The colour is assigned here rather than left blank because it is
/// mandatory by design: it is the mark that keeps one client's context from
/// being read as another's, and a blank one is a mark nobody sees.
const DEFAULT_WORKSPACE: &str = "tw_personal";
const DEFAULT_ACCENT: &str = "#6f8fbf";

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

impl Store {
    fn ensure_default_workspace(&self) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO trust_workspace \
             (id, slug, label, color, vault_namespace, is_default, created_at) \
             VALUES (?1, 'personal', 'Personal', ?2, 'personal', 1, ?3)",
            rusqlite::params![DEFAULT_WORKSPACE, DEFAULT_ACCENT, now()],
        )?;
        Ok(())
    }

    /// Every project, most recently opened first.
    ///
    /// Ordering by use rather than by name: the list is navigated, not read,
    /// and what you touched last is what you reach for next. Ties fall back to
    /// creation order so the list never reshuffles on its own.
    pub fn projects(&self) -> Result<Vec<ProjectRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.name, p.root_path, p.group_name, w.color, \
                    p.origin_url, p.last_opened_at, p.icon, p.color \
             FROM project p JOIN trust_workspace w ON w.id = p.trust_workspace_id \
             WHERE p.archived_at IS NULL \
             ORDER BY p.last_opened_at DESC NULLS LAST, p.created_at ASC",
        )?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ProjectRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    root_path: row.get(2)?,
                    group: row.get(3)?,
                    accent: row.get(4)?,
                    origin: row.get(5)?,
                    last_opened_at: row.get(6)?,
                    icon: row.get(7)?,
                    color: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn project(&self, id: &str) -> Result<Option<ProjectRow>, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT p.id, p.name, p.root_path, p.group_name, w.color, \
                        p.origin_url, p.last_opened_at, p.icon, p.color \
                 FROM project p JOIN trust_workspace w ON w.id = p.trust_workspace_id \
                 WHERE p.id = ?1",
                [id],
                |row| {
                    Ok(ProjectRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        root_path: row.get(2)?,
                        group: row.get(3)?,
                        accent: row.get(4)?,
                        origin: row.get(5)?,
                        last_opened_at: row.get(6)?,
                        icon: row.get(7)?,
                        color: row.get(8)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Registers a folder as a project, or returns the one already registered.
    ///
    /// Idempotent because `project.root_path` is unique and adding the same
    /// folder twice is a thing people do — the second attempt should open it,
    /// not raise a constraint error at them.
    pub fn add_project(&self, root: &Path, origin_url: Option<&str>) -> Result<String, StoreError> {
        self.ensure_default_workspace()?;

        let root_path = root.to_string_lossy().into_owned();
        if let Some(existing) = self
            .conn
            .query_row(
                "SELECT id FROM project WHERE root_path = ?1",
                [&root_path],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(existing);
        }

        let name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root_path.clone());

        let id = format!("prj_{}", ulid::Ulid::generate());
        // Named once, here, from the name it is born with: a rename later
        // must not move a folder a conversation may be open in.
        let taken = self.taken_folders()?;
        let folder = crate::home::folder_for(&name, &id, |one| taken.contains(one));
        self.conn.execute(
            "INSERT INTO project \
             (id, trust_workspace_id, name, root_path, origin_url, origin_hash, created_at, last_opened_at, folder) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8)",
            rusqlite::params![
                id,
                DEFAULT_WORKSPACE,
                name,
                root_path,
                origin_url,
                origin_url.map(origin_hash),
                now(),
                folder,
            ],
        )?;

        Ok(id)
    }

    /// Takes a project out of the list without touching the folder.
    ///
    /// Archived rather than deleted: the board, the cards and their history
    /// hang off this row, and dropping it would take work with it that the
    /// person only asked to stop seeing.
    pub fn forget_project(&self, id: &str) -> Result<bool, StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs() as i64)
            .unwrap_or_default();
        let changed = self.conn.execute(
            "UPDATE project SET archived_at = ?2, revision = revision + 1 \
             WHERE id = ?1 AND archived_at IS NULL",
            rusqlite::params![id, now],
        )?;
        Ok(changed > 0)
    }

    /// Erases a project and everything hanging off it.
    ///
    /// The other half of `forget_project`, and only ever reached by ticking a
    /// box that says so: the board, its cards, their runs and the notes all
    /// reference this row and go with it. The folder on disk is still never
    /// touched — this is the row, not the repository.
    pub fn erase_project(&self, id: &str) -> Result<bool, StoreError> {
        let changed = self
            .conn
            .execute("DELETE FROM project WHERE id = ?1", [id])?;
        Ok(changed > 0)
    }

    /// Renames a project without touching the folder it points at.
    ///
    /// The name is the app's, not git's: a folder called `api-v2-final` can be
    /// called API here, and renaming the folder to match is not something an
    /// editor should do to somebody's checkout.
    pub fn rename_project(&self, id: &str, name: &str) -> Result<bool, StoreError> {
        let changed = self.conn.execute(
            "UPDATE project SET name = ?2, revision = revision + 1 \
             WHERE id = ?1 AND archived_at IS NULL",
            rusqlite::params![id, name],
        )?;
        Ok(changed > 0)
    }

    /// Everything the edit dialog sets at once: the name, the group it is
    /// listed under, and its mark. `None` clears a group, icon or colour.
    pub fn edit_project(
        &self,
        id: &str,
        name: &str,
        group: Option<&str>,
        icon: Option<&str>,
        color: Option<&str>,
    ) -> Result<bool, StoreError> {
        let changed = self.conn.execute(
            "UPDATE project SET name = ?2, group_name = ?3, icon = ?4, color = ?5, \
             revision = revision + 1 WHERE id = ?1 AND archived_at IS NULL",
            rusqlite::params![id, name, group, icon, color],
        )?;
        Ok(changed > 0)
    }

    /// Marks a project as the one being worked in, which is what orders the list.
    pub fn touch_project(&self, id: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE project SET last_opened_at = ?2 WHERE id = ?1",
            rusqlite::params![id, now()],
        )?;
        Ok(())
    }
}

/// The normalised git origin, hashed.
///
/// Keyed on this rather than on a path so two clones of the same repository on
/// different machines agree about which project they are. Normalisation is
/// what makes that true: `git@host:org/repo.git` and
/// `https://host/org/repo` are the same repository and must hash the same.
fn origin_hash(url: &str) -> String {
    use sha2::{Digest, Sha256};

    let normalised = url
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .replace("git@", "")
        .replace("ssh://", "")
        .replace("https://", "")
        .replace("http://", "")
        .replacen(':', "/", 1)
        .to_lowercase();

    let digest = Sha256::digest(normalised.as_bytes());
    digest
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
#[path = "projects_tests.rs"]
mod tests;
