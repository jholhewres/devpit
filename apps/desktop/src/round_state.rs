//! Where an orchestrator's round stands, kept by devpit in its own folder.
//!
//! The chat compacts several times a day, and after each it knew only what
//! the summary kept. devpit knows the round itself — every session, what each
//! is waiting on, the drafts, the reminders — so it writes `context/now.md` at
//! each of the person's turns and when the orchestrator lists its sessions,
//! and the brief says to read it first. Rewritten whole, never appended:
//! `sessions.md` is the history, this is the present.

use std::path::{Path, PathBuf};

use devpit_rpc::{EndedSession, LiveSession};

/// The ended sessions worth naming: the latest few, not a history.
const ENDED: usize = 8;

/// A reminder as the round shows it.
pub(crate) struct Reminder {
    pub project: String,
    pub title: String,
    /// Seconds since the epoch.
    pub at: i64,
    pub went_off: bool,
}

/// The file, inside the orchestrator's folder.
pub(crate) fn file(folder: &Path) -> PathBuf {
    folder.join("context").join("now.md")
}

/// `epoch` seconds as a moment a reader can place.
fn stamp(epoch: i64) -> String {
    let day = devpit_agentcli::spend_scan::day_of(epoch);
    let of_day = epoch.rem_euclid(86_400);
    format!("{day} {:02}:{:02} UTC", of_day / 3600, of_day % 3600 / 60)
}

/// The round, as the file says it.
pub(crate) fn rendered(
    now: i64,
    sessions: &[LiveSession],
    ended: &[EndedSession],
    reminders: &[Reminder],
) -> String {
    let mut out = format!(
        "# Now\n\nWritten by devpit at {}; rewritten at every turn of the person's. Read it, do not edit it.\n",
        stamp(now)
    );
    let waiting: Vec<&LiveSession> = sessions
        .iter()
        .filter(|one| one.waiting.is_some())
        .collect();
    let drafted: Vec<&LiveSession> = sessions.iter().filter(|one| one.draft.is_some()).collect();
    out.push_str("\n## Waiting on the person\n\n");
    if waiting.is_empty() && drafted.is_empty() && !reminders.iter().any(|one| one.went_off) {
        out.push_str("Nothing.\n");
    }
    for one in &waiting {
        let question = one
            .waiting
            .as_ref()
            .map(|asked| asked.question.as_str())
            .unwrap_or_default();
        out.push_str(&format!("- {} asks: {}\n", one.name, line(question)));
    }
    for one in &drafted {
        let draft = one.draft.as_deref().unwrap_or_default();
        out.push_str(&format!(
            "- a draft for {} waits to be sent: «{}»\n",
            one.name,
            line(draft)
        ));
    }
    for one in reminders.iter().filter(|one| one.went_off) {
        out.push_str(&format!(
            "- reminder went off ({}): {}\n",
            one.project, one.title
        ));
    }
    out.push_str(&format!("\n## Sessions running ({})\n\n", sessions.len()));
    if sessions.is_empty() {
        out.push_str("None.\n");
    }
    for one in sessions {
        let mut said = format!(
            "- {} — {}",
            one.name,
            one.project_name.as_deref().unwrap_or("no project")
        );
        if let Some(card) = &one.card_id {
            said.push_str(&format!(", card {card}"));
        }
        said.push_str(&format!(", {}", one.status));
        if let Some(step) = &one.step {
            said.push_str(&format!(" ({})", line(step)));
        }
        if let Some(since) = one.since {
            said.push_str(&format!(", since {}", stamp((since / 1000.0) as i64)));
        }
        out.push_str(&said);
        out.push('\n');
    }
    out.push_str("\n## Reminders to come\n\n");
    let coming: Vec<&Reminder> = reminders.iter().filter(|one| !one.went_off).collect();
    if coming.is_empty() {
        out.push_str("None.\n");
    }
    for one in coming {
        out.push_str(&format!(
            "- {} ({}): {}\n",
            stamp(one.at),
            one.project,
            one.title
        ));
    }
    out.push_str("\n## Ended lately\n\n");
    if ended.is_empty() {
        out.push_str("None.\n");
    }
    for one in ended.iter().take(ENDED) {
        let mut said = format!(
            "- {} — {}, ended {}",
            one.name,
            one.project_name.as_deref().unwrap_or("no project"),
            stamp((one.ended_at / 1000.0) as i64)
        );
        if let Some(by) = &one.ended_by {
            said.push_str(&format!(", stopped by the {by}"));
        }
        if let Some(dirty) = one.dirty.filter(|count| *count > 0) {
            said.push_str(&format!(", {dirty} files not committed"));
        }
        out.push_str(&said);
        out.push('\n');
    }
    out
}

/// One line of something that may have many.
fn line(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut cut: String = flat.chars().take(160).collect();
    if flat.chars().count() > 160 {
        cut.push('…');
    }
    cut
}

/// Writes the round of the orchestrator in `folder`, speaking as `profile`,
/// whose reach is `sessions`. A failure costs only the file.
pub(crate) fn keep(folder: &Path, here_id: &str, profile: &str, sessions: &[LiveSession]) {
    let linked = crate::orchestrator_links::linked(folder);
    let in_reach =
        |id: Option<&str>| id.is_some_and(|id| id == here_id || linked.iter().any(|one| one == id));
    let ended: Vec<EndedSession> = crate::ended_sessions::ended_now(profile)
        .map(|listed| listed.sessions)
        .unwrap_or_default()
        .into_iter()
        .filter(|one| in_reach(one.project_id.as_deref()))
        .collect();
    let reminders = reminders(&|id| in_reach(Some(id)));
    let text = rendered(
        devpit_core::reports::now() as i64,
        sessions,
        &ended,
        &reminders,
    );
    let target = file(folder);
    let written = target
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|_| {
            let temporary = target.with_extension("md.writing");
            std::fs::write(&temporary, text)?;
            std::fs::rename(&temporary, &target)
        });
    if let Err(err) = written {
        devpit_core::reports::background("round state", &err);
    }
}

/// The open reminders of the projects in reach, with their project's name.
fn reminders(in_reach: &dyn Fn(&str) -> bool) -> Vec<Reminder> {
    let Ok(store) = crate::projects::store() else {
        return Vec::new();
    };
    let names = crate::projects::project_list_unread_now()
        .map(|listed| listed.projects)
        .unwrap_or_default();
    store
        .reminders_open(None)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| in_reach(&row.project_id))
        .map(|row| Reminder {
            project: names
                .iter()
                .find(|one| one.id == row.project_id)
                .map(|one| one.name.clone())
                .unwrap_or_default(),
            title: row.title,
            at: row.due_at,
            went_off: row.reminded_at.is_some(),
        })
        .collect()
}

#[cfg(test)]
#[path = "round_state_tests.rs"]
mod tests;
