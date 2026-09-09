//! The answers the first run asks for, and anything else the person chooses.
//!
//! Key/value rather than a column per setting: a new preference would otherwise
//! be a migration every time, and a migration over real data is the part that
//! goes wrong.

use crate::store::{Store, StoreError};

/// Keys, in one place.
///
/// A key spelled differently in two files is a setting that silently forgets
/// itself, and nothing about the symptom points at the typo.
pub mod key {
    pub const TELEMETRY: &str = "telemetry.enabled";
    pub const ONBOARDED_AT: &str = "onboarding.completed_at";
    pub const THEME: &str = "appearance.theme";
    pub const AUTO_UPDATE: &str = "general.automatic_updates";
    pub const KEEP_TRANSCRIPTS: &str = "storage.keep_transcripts";
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

impl Store {
    pub fn preference(&self, key: &str) -> Result<Option<String>, StoreError> {
        use rusqlite::OptionalExtension;
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM preference WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Writes a preference, bumping its revision.
    ///
    /// The revision is what a later sync compares; incrementing on write rather
    /// than at sync time means a value changed twice offline still says so.
    pub fn set_preference(&self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO preference (key, value, updated_at) VALUES (?1, ?2, ?3) \
             ON CONFLICT(key) DO UPDATE SET \
               value = excluded.value, \
               updated_at = excluded.updated_at, \
               revision = preference.revision + 1",
            rusqlite::params![key, value, now()],
        )?;
        Ok(())
    }

    /// Reads a preference that is a yes/no.
    ///
    /// Three states, not two: unset is neither yes nor no. Telemetry that has
    /// never been asked about must not default to on, and a `bool` has nowhere
    /// to say "not asked yet".
    pub fn preference_flag(&self, key: &str) -> Result<Option<bool>, StoreError> {
        Ok(self.preference(key)?.map(|value| value == "true"))
    }

    pub fn set_preference_flag(&self, key: &str, value: bool) -> Result<(), StoreError> {
        self.set_preference(key, if value { "true" } else { "false" })
    }
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
    fn an_unset_flag_is_neither_true_nor_false() {
        // The reason it is an Option. Defaulting telemetry to `false` would
        // read as "they said no" and the screen would never ask.
        let (_dir, store) = store();
        assert_eq!(store.preference_flag(key::TELEMETRY).expect("read"), None);
    }

    #[test]
    fn writing_twice_keeps_the_last_value_and_counts_the_change() {
        let (_dir, store) = store();
        store
            .set_preference_flag(key::TELEMETRY, true)
            .expect("write");
        store
            .set_preference_flag(key::TELEMETRY, false)
            .expect("write");

        assert_eq!(
            store.preference_flag(key::TELEMETRY).expect("read"),
            Some(false)
        );

        let revision: i64 = store
            .conn()
            .query_row(
                "SELECT revision FROM preference WHERE key = ?1",
                [key::TELEMETRY],
                |row| row.get(0),
            )
            .expect("read revision");
        assert_eq!(revision, 2, "the second write did not bump the revision");
    }

    #[test]
    fn a_value_survives_reopening_the_database() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("state.db");

        let store = Store::open(&path).expect("open");
        store
            .set_preference_flag(key::TELEMETRY, true)
            .expect("write");
        drop(store);

        let store = Store::open(&path).expect("reopen");
        assert_eq!(
            store.preference_flag(key::TELEMETRY).expect("read"),
            Some(true)
        );
    }
}
