//! The pane tree a tab shows, stored as JSON the contract owns.
//!
//! Keyed by tab and not by project: one tree per project meant opening a
//! second terminal tab had to split the first one's tree to get a leaf, so the
//! tree's shape said nothing about what the person had arranged. Keyed by tab,
//! a split is dividing what one tab shows — which is what the word means.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

impl Store {
    pub fn pane_layout(
        &self,
        project_id: &str,
        tab_id: &str,
    ) -> Result<Option<(String, String)>, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT tree, focused_id FROM pane_layout WHERE project_id = ?1 AND tab_id = ?2",
                [project_id, tab_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        Ok(row)
    }

    /// Every tab that has a layout in this project, oldest first.
    ///
    /// The window asks on open: a tab strip restored from local storage names
    /// tabs that may have lost their tree, and a tree whose tab is gone is
    /// a session nobody can reach.
    pub fn pane_layout_tabs(&self, project_id: &str) -> Result<Vec<String>, StoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT tab_id FROM pane_layout WHERE project_id = ?1 ORDER BY updated_at")?;
        let rows = statement.query_map([project_id], |row| row.get::<_, String>(0))?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    /// Forgets a tab's tree. A tab that never had one is not an error.
    pub fn forget_pane_layout(&self, project_id: &str, tab_id: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM pane_layout WHERE project_id = ?1 AND tab_id = ?2",
            [project_id, tab_id],
        )?;
        Ok(())
    }

    pub fn set_pane_layout(
        &self,
        project_id: &str,
        tab_id: &str,
        tree: &str,
        focused_id: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO pane_layout (project_id, tab_id, tree, focused_id, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(project_id, tab_id) DO UPDATE SET
                tree = excluded.tree,
                focused_id = excluded.focused_id,
                updated_at = excluded.updated_at",
            rusqlite::params![project_id, tab_id, tree, focused_id, now()],
        )?;
        Ok(())
    }
}

/// A tab a card named, and the tree in it.
pub struct CardTabLayout {
    pub project_id: String,
    pub tab_id: String,
    pub tree: String,
    pub focused_id: String,
}

impl Store {
    /// Every layout whose tab a card named, across projects.
    ///
    /// `_` is a wildcard to LIKE, so the prefix is escaped: `tabXcardY` is not
    /// a card's tab.
    pub fn card_tab_layouts(&self) -> Result<Vec<CardTabLayout>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT project_id, tab_id, tree, focused_id FROM pane_layout \
             WHERE tab_id LIKE 'tab\\_card\\_%' ESCAPE '\\'",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(CardTabLayout {
                project_id: row.get(0)?,
                tab_id: row.get(1)?,
                tree: row.get(2)?,
                focused_id: row.get(3)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
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

        assert!(store.pane_layout(&id, "tab_a").expect("read").is_none());

        store
            .set_pane_layout(&id, "tab_a", r#"{"type":"leaf"}"#, "leaf_a")
            .expect("write");
        let (tree, focused) = store
            .pane_layout(&id, "tab_a")
            .expect("read")
            .expect("present");
        assert_eq!(tree, r#"{"type":"leaf"}"#);
        assert_eq!(focused, "leaf_a");

        store
            .set_pane_layout(&id, "tab_a", r#"{"type":"split"}"#, "leaf_b")
            .expect("overwrite");
        let (tree, focused) = store
            .pane_layout(&id, "tab_a")
            .expect("read")
            .expect("present");
        assert_eq!(tree, r#"{"type":"split"}"#);
        assert_eq!(focused, "leaf_b");
    }

    #[test]
    fn two_tabs_of_one_project_keep_two_trees() {
        /* The whole point of the key. One tree per project meant a second tab
        had to split the first one's, so the shape said nothing. */
        let (dir, store) = store();
        let root = dir.path().join("project");
        std::fs::create_dir_all(&root).expect("create");
        let id = store.add_project(Path::new(&root), None).expect("add");

        store
            .set_pane_layout(&id, "tab_a", "A", "leaf_a")
            .expect("a");
        store
            .set_pane_layout(&id, "tab_b", "B", "leaf_b")
            .expect("b");

        assert_eq!(
            store.pane_layout(&id, "tab_a").expect("read").expect("a").0,
            "A"
        );
        assert_eq!(
            store.pane_layout(&id, "tab_b").expect("read").expect("b").0,
            "B"
        );
        assert_eq!(store.pane_layout_tabs(&id).expect("tabs").len(), 2);

        store.forget_pane_layout(&id, "tab_a").expect("forget");
        assert!(store.pane_layout(&id, "tab_a").expect("read").is_none());
        assert!(store.pane_layout(&id, "tab_b").expect("read").is_some());
    }
}
