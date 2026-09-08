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
    Migration {
        version: 4,
        sql: r#"
-- A column is data, not code. It is created, renamed, reordered and deleted
-- by the person using it, and nothing in the code may assume the names the
-- default board happens to seed.
CREATE TABLE board_column (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    position    INTEGER NOT NULL,
    -- NULL means the column runs nothing, and that is a column doing its job:
    -- a holding lane is not half-configured.
    step_id     TEXT REFERENCES step(id) ON DELETE SET NULL,
    created_at  INTEGER NOT NULL
);
CREATE UNIQUE INDEX board_column_position ON board_column(project_id, position);

-- A step belongs to the project, not to one column: "run the tests" serves
-- more than one lane, and editing the command in one place holds everywhere.
--
-- `config` is JSON, shaped by `kind`:
--   agent   -> { agent, schema, budgetUsd, model }
--   session -> { agent?, worktree, model }
--   command -> { command, timeoutSeconds }
CREATE TABLE step (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('agent','session','command')),
    name        TEXT NOT NULL,
    config      TEXT NOT NULL,
    -- A deploy has no undo. An irreversible step is confirmed before it runs,
    -- never fired by a drag alone.
    irreversible INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL
);

-- RESTRICT, not CASCADE: deleting a column that still holds cards has to fail
-- so the interface can ask where they go. CASCADE would delete described work
-- on one click.
CREATE TABLE card (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    column_id   TEXT NOT NULL REFERENCES board_column(id) ON DELETE RESTRICT,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL DEFAULT '',
    position    INTEGER NOT NULL,
    -- The worktree of this line of work, and the ref it started from. The
    -- diff is against `base_ref`, never against HEAD.
    worktree_path TEXT,
    base_ref    TEXT,
    archived_at INTEGER,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
CREATE INDEX card_column ON card(column_id, position);

CREATE TABLE run (
    id          TEXT PRIMARY KEY,
    card_id     TEXT NOT NULL REFERENCES card(id) ON DELETE CASCADE,
    step_id     TEXT NOT NULL REFERENCES step(id) ON DELETE RESTRICT,
    state       TEXT NOT NULL
                CHECK (state IN ('running','ok','failed','cancelled')),
    -- Validated JSON for an agent step, streamed output for a command, or the
    -- reason it failed.
    output      TEXT,
    exit_code   INTEGER,
    cost_usd    REAL,
    duration_ms INTEGER,
    started_at  INTEGER NOT NULL,
    ended_at    INTEGER
);
CREATE INDEX run_card ON run(card_id, started_at DESC);

-- Only the link. Whether a session is idle or busy, and what it cost, are
-- answered by the agent CLI and its transcript; a copy here would be a second
-- truth that drifts from the first.
CREATE TABLE session_link (
    card_id     TEXT PRIMARY KEY REFERENCES card(id) ON DELETE CASCADE,
    short_id    TEXT NOT NULL,
    session_id  TEXT NOT NULL,
    transcript_path TEXT,
    created_at  INTEGER NOT NULL
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

/// Applies only up to `version`, to build a database of an older shape.
///
/// Only the test asks: it is how "the migration is additive" is proved against
/// rows rather than against an empty file.
#[cfg(test)]
pub fn run_up_to(conn: &Connection, version: i64) -> Result<(), StoreError> {
    for migration in MIGRATIONS.iter().filter(|m| m.version <= version) {
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
