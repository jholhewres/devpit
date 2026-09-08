//! Schema migrations, applied once each.
//!
//! Idempotency matters: the app opens the store on every start, and a
//! migration that reruns against real data loses data. The applied version is
//! read from the database itself (`user_version`), not tracked beside it.

use rusqlite::Connection;

use crate::store::StoreError;

struct Migration {
    version: i64,
    sql: &'static str,
}

/// Migration 001 — the core tables.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: r#"
CREATE TABLE trust_workspace (
    id          TEXT PRIMARY KEY,
    slug        TEXT NOT NULL UNIQUE,
    label       TEXT NOT NULL,
    -- Required, not decorative: the persistent visual mark in the bar. Without
    -- it, client context eventually lands in the personal remote.
    color       TEXT NOT NULL,
    memory_remote    TEXT,
    vault_namespace  TEXT NOT NULL,
    is_default  INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL,
    revision    INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE project (
    id          TEXT PRIMARY KEY,
    trust_workspace_id TEXT NOT NULL REFERENCES trust_workspace(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    -- Local. Never syncs: an absolute path means nothing on another machine.
    root_path   TEXT NOT NULL,
    origin_url  TEXT,
    -- Normalized, hashed git origin. Routing key across machines: two clones
    -- of the same repo converge on the same remote project.
    origin_hash TEXT,
    group_name  TEXT,
    last_opened_at INTEGER,
    archived_at INTEGER,
    created_at  INTEGER NOT NULL,
    revision    INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX project_root_path ON project(root_path);
CREATE INDEX project_origin_hash ON project(origin_hash) WHERE origin_hash IS NOT NULL;

CREATE TABLE scratch (
    id          TEXT PRIMARY KEY,
    project_id  TEXT REFERENCES project(id) ON DELETE CASCADE,
    body        TEXT NOT NULL,
    -- Lets the system age a note: "open 40 days, the file it cites changed 6
    -- times, still valid?". The antidote to orphan notes is the system asking,
    -- not discipline.
    anchor_path   TEXT,
    anchor_commit TEXT,
    archived_at INTEGER,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    revision    INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX scratch_project ON scratch(project_id, archived_at);

CREATE TABLE drawing (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    -- Opaque on purpose: the core never reads it. Storing the editor's format
    -- as structured data would sign us up to maintain it every release.
    scene       TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    revision    INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX drawing_name ON drawing(project_id, name);

-- Append-only. Nothing here is updated, which is what makes it auditable.
CREATE TABLE event (
    id          TEXT PRIMARY KEY,
    topic       TEXT NOT NULL,
    scope_id    TEXT,
    -- neutral | attention | failure | verified. A field of the event, not a UI
    -- decision: colouring normal activity turns the screen into an alarm panel.
    severity    TEXT NOT NULL,
    payload     TEXT NOT NULL,
    acknowledged_at INTEGER,
    created_at  INTEGER NOT NULL
);

CREATE INDEX event_feed ON event(created_at DESC);
"#,
    },
    // Migration 002 — the answers the first run asks for.
    Migration {
        version: 2,
        sql: r#"
-- Key/value and not a column per setting: every new preference would otherwise
-- be a migration, and a migration over real data is the part that goes wrong.
-- Values are text; the reader parses what it asked for.
CREATE TABLE preference (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  INTEGER NOT NULL,
    revision    INTEGER NOT NULL DEFAULT 1
);
"#,
    },
    Migration {
        version: 3,
        sql: r#"
CREATE TABLE pane_layout (
    project_id  TEXT PRIMARY KEY REFERENCES project(id) ON DELETE CASCADE,
    tree        TEXT NOT NULL,
    focused_id  TEXT NOT NULL,
    updated_at  INTEGER NOT NULL
);
"#,
    },
];

/// Applies whatever has not been applied yet. Called on every start.
pub fn run(conn: &Connection) -> Result<(), StoreError> {
    let applied: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    for migration in MIGRATIONS.iter().filter(|m| m.version > applied) {
        // One transaction per migration: a partial failure must not leave half
        // the tables standing with the version already bumped.
        conn.execute_batch(&format!(
            "BEGIN; {} PRAGMA user_version = {}; COMMIT;",
            migration.sql, migration.version
        ))?;
    }

    Ok(())
}

/// Only the test asks. Same rule as the RPC contract: no caller, no code.
#[cfg(test)]
pub fn latest_version() -> i64 {
    MIGRATIONS.last().map_or(0, |m| m.version)
}
