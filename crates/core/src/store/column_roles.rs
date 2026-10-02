//! What a column is for: waiting work, work in progress, work to check, done.
//!
//! Every board names its columns its own way — inbox, todo, doing, review,
//! ship — and an agent moving its card along needs to know which column holds
//! what, whatever it is called. The person can say; until they do, the name
//! is read, in the few languages column names usually come in.

use crate::store::{Store, StoreError};

/// The roles a column can have.
pub const ROLES: [&str; 4] = ["backlog", "doing", "check", "done"];

/// The role a column's name suggests, if it suggests one.
pub fn role_from_name(name: &str) -> Option<&'static str> {
    let name = name.trim().to_lowercase();
    let is = |words: &[&str]| words.iter().any(|word| name == *word);
    let has = |words: &[&str]| words.iter().any(|word| name.contains(word));
    if is(&[
        "inbox",
        "todo",
        "to do",
        "backlog",
        "next",
        "ideas",
        "refine",
        "ready",
        "a fazer",
        "pendente",
        "pendentes",
    ]) || has(&["backlog", "to do", "todo"])
    {
        return Some("backlog");
    }
    if is(&[
        "doing",
        "wip",
        "in progress",
        "progress",
        "fazendo",
        "em andamento",
        "andamento",
        "working",
    ]) || has(&["in progress", "doing", "wip", "andamento"])
    {
        return Some("doing");
    }
    if is(&[
        "check",
        "review",
        "qa",
        "test",
        "testing",
        "verify",
        "revisão",
        "revisar",
        "conferir",
        "validação",
    ]) || has(&["review", "check", "qa", "revis"])
    {
        return Some("check");
    }
    if is(&[
        "done",
        "ship",
        "shipped",
        "closed",
        "complete",
        "completed",
        "feito",
        "concluído",
        "pronto",
        "entregue",
    ]) || has(&["done", "shipped", "conclu"])
    {
        return Some("done");
    }
    None
}

/// What a column is for: the person's choice, or what its name suggests.
pub fn role_of(chosen: Option<&str>, name: &str) -> Option<&'static str> {
    match chosen {
        Some(role) => ROLES.iter().find(|one| **one == role).copied(),
        None => role_from_name(name),
    }
}

impl Store {
    /// Says what a column is for, or `None` to go back to its name.
    pub fn set_column_role(&self, column_id: &str, role: Option<&str>) -> Result<bool, StoreError> {
        if role.is_some_and(|role| !ROLES.contains(&role)) {
            return Ok(false);
        }
        let changed = self.conn.execute(
            "UPDATE board_column SET role = ?2 WHERE id = ?1",
            rusqlite::params![column_id, role],
        )?;
        Ok(changed > 0)
    }
}

#[cfg(test)]
#[path = "column_roles_tests.rs"]
mod tests;
