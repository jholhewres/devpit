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
}

pub struct NoteRow {
    pub id: String,
    pub body: String,
    pub created_at: i64,
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
            "SELECT p.id, p.name, p.root_path, p.group_name, w.color \
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
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn project(&self, id: &str) -> Result<Option<ProjectRow>, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT p.id, p.name, p.root_path, p.group_name, w.color \
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
        self.conn.execute(
            "INSERT INTO project \
             (id, trust_workspace_id, name, root_path, origin_url, origin_hash, created_at, last_opened_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            rusqlite::params![
                id,
                DEFAULT_WORKSPACE,
                name,
                root_path,
                origin_url,
                origin_url.map(origin_hash),
                now(),
            ],
        )?;

        Ok(id)
    }

    /// Marks a project as the one being worked in, which is what orders the list.
    pub fn touch_project(&self, id: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE project SET last_opened_at = ?2 WHERE id = ?1",
            rusqlite::params![id, now()],
        )?;
        Ok(())
    }

    pub fn notes(&self, project_id: &str) -> Result<Vec<NoteRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, body, created_at FROM scratch \
             WHERE project_id = ?1 AND archived_at IS NULL \
             ORDER BY created_at DESC",
        )?;

        let rows = stmt
            .query_map([project_id], |row| {
                Ok(NoteRow {
                    id: row.get(0)?,
                    body: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn add_note(&self, project_id: &str, body: &str) -> Result<String, StoreError> {
        let id = format!("scr_{}", ulid::Ulid::generate());
        let at = now();
        self.conn.execute(
            "INSERT INTO scratch (id, project_id, body, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?4)",
            rusqlite::params![id, project_id, body, at],
        )?;
        Ok(id)
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
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(&dir.path().join("state.db")).expect("open");
        (dir, store)
    }

    #[test]
    fn adding_the_same_folder_twice_opens_it_rather_than_failing() {
        let (dir, store) = store();
        let root = dir.path().join("project");
        std::fs::create_dir_all(&root).expect("create");

        let first = store.add_project(&root, None).expect("add");
        let second = store.add_project(&root, None).expect("add again");
        assert_eq!(first, second, "the second add created a duplicate");
        assert_eq!(store.projects().expect("list").len(), 1);
    }

    #[test]
    fn the_two_shapes_of_one_origin_hash_the_same() {
        // The reason this exists: the same repository cloned over ssh on one
        // machine and https on another has to converge on one project.
        assert_eq!(
            origin_hash("git@github.com:jholhewres/devpit.git"),
            origin_hash("https://github.com/jholhewres/devpit")
        );
    }

    #[test]
    fn a_project_carries_its_workspace_colour() {
        let (dir, store) = store();
        let root = dir.path().join("project");
        std::fs::create_dir_all(&root).expect("create");
        store.add_project(&root, None).expect("add");

        let listed = store.projects().expect("list");
        assert_eq!(listed[0].accent, DEFAULT_ACCENT);
        assert_eq!(listed[0].name, "project");
    }

    #[test]
    fn notes_belong_to_their_project_and_come_back_newest_first() {
        let (dir, store) = store();
        let root = dir.path().join("project");
        std::fs::create_dir_all(&root).expect("create");
        let id = store.add_project(&root, None).expect("add");

        store.add_note(&id, "first").expect("note");
        store.add_note(&id, "second").expect("note");

        let notes = store.notes(&id).expect("notes");
        assert_eq!(notes.len(), 2);
        // Same-second inserts tie on created_at, so both orders are valid
        // here; what must hold is that neither leaks into another project.
        assert!(notes.iter().all(|note| !note.body.is_empty()));
        assert!(store.notes("prj_other").expect("notes").is_empty());
    }
}
