//! The pane tree for a project, stored as JSON the contract owns.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

impl Store {
    pub fn pane_layout(&self, project_id: &str) -> Result<Option<(String, String)>, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT tree, focused_id FROM pane_layout WHERE project_id = ?1",
                [project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        Ok(row)
    }

    pub fn set_pane_layout(
        &self,
        project_id: &str,
        tree: &str,
        focused_id: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO pane_layout (project_id, tree, focused_id, updated_at) \
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id) DO UPDATE SET
                tree = excluded.tree,
                focused_id = excluded.focused_id,
                updated_at = excluded.updated_at",
            rusqlite::params![project_id, tree, focused_id, now()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::store::Store;
    use std::path::Path;

    fn store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(&dir.path().join("state.db")).expect("open");
        (dir, store)
    }

    #[test]
    fn layout_round_trips_and_overwrites() {
        let (dir, store) = store();
        let root = dir.path().join("project");
        std::fs::create_dir_all(&root).expect("create");
        let id = store.add_project(Path::new(&root), None).expect("add");

        assert!(store.pane_layout(&id).expect("read").is_none());

        store
            .set_pane_layout(&id, r#"{"type":"leaf"}"#, "leaf_a")
            .expect("write");
        let (tree, focused) = store.pane_layout(&id).expect("read").expect("present");
        assert_eq!(tree, r#"{"type":"leaf"}"#);
        assert_eq!(focused, "leaf_a");

        store
            .set_pane_layout(&id, r#"{"type":"split"}"#, "leaf_b")
            .expect("overwrite");
        let (tree, focused) = store.pane_layout(&id).expect("read").expect("present");
        assert_eq!(tree, r#"{"type":"split"}"#);
        assert_eq!(focused, "leaf_b");
    }
}
