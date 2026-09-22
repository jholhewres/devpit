//! One card, opened: its description, its deadline, its conversation, the
//! files pinned to it and the checkout its work happens in.
//!
//! Apart from `board.rs` because that file answers "what does the board look
//! like" and this one answers "what is this card". The board reads thirty
//! cards to draw thirty tiles; this reads one, and can afford to ask git.

use std::path::{Path, PathBuf};

use devpit_core::{limits, AttachmentRow, CommentRow, Store};
use devpit_rpc::{CardDetail, Checkout, Comment, ErrorCode, Pinned, RpcError};

use crate::board::{card_of, steps_of};
use crate::projects::{locate, store};

/// Who a comment is from when a person writes it.
///
/// A fixed word rather than a name: there is one person at this keyboard, the
/// screen knows what to call them, and a name stored in a row is a name that
/// is wrong the day they change it.
pub const YOU: &str = "you";

fn seconds(at: i64) -> f64 {
    at as f64
}

/// The file a pin points at, as it stands now.
///
/// Asked of the disk on every read rather than recorded once: a pin is a path,
/// and a path is a promise about somebody else's filesystem. Recording "it
/// exists" would be recording something that stops being true without telling
/// anyone.
pub(crate) fn pinned_now(row: AttachmentRow) -> Pinned {
    let path = Path::new(&row.path);
    let seen = std::fs::metadata(path).ok();
    Pinned {
        exists: seen.is_some(),
        bytes: seen.map(|meta| meta.len() as f64),
        id: row.id,
        path: row.path,
        label: row.label,
        created_at: seconds(row.created_at),
        plugin: row.plugin_id,
    }
}

fn spoken(row: CommentRow) -> Comment {
    Comment {
        id: row.id,
        author: row.author,
        body: row.body,
        created_at: seconds(row.created_at),
        edited_at: row.edited_at.map(seconds),
    }
}

/// What git says about the card's checkout, when it has one.
fn checkout_of(row: &devpit_core::CardRow) -> Option<Checkout> {
    let path = row.worktree_path.clone()?;
    let at = PathBuf::from(&path);
    if !at.is_dir() {
        // Still reported, marked absent: a card that lost its folder has to
        // say so, or the button to open it is a button that fails.
        return Some(Checkout {
            path,
            branch: None,
            base_ref: row.base_ref.clone(),
            dirty_files: None,
            exists: false,
        });
    }
    let read = devpit_git::status(&at).ok();
    Some(Checkout {
        path,
        branch: read.as_ref().map(|seen| seen.branch.clone()),
        base_ref: row.base_ref.clone(),
        dirty_files: read.as_ref().map(|seen| seen.dirty_files()),
        exists: true,
    })
}

/// `card.detail` — everything one card is.
///
/// Off the main thread: it asks git, tmux, `ps` and sometimes the CLI, and an
/// open card reads it again whenever one of its sessions says something.
#[tauri::command]
#[specta::specta]
pub async fn card_detail(project_id: String, card_id: String) -> Result<CardDetail, RpcError> {
    tauri::async_runtime::spawn_blocking(move || detail_of(project_id, card_id))
        .await
        .map_err(|err| RpcError::internal(err.to_string()))?
}

/// Everything one card is, read on the caller's thread.
pub(crate) fn detail_of(project_id: String, card_id: String) -> Result<CardDetail, RpcError> {
    let store = store()?;
    let steps = steps_of(&store, &project_id)?;
    let mut card = card_of(&store, &card_id, &steps)?;
    // The CLI is asked only when the card has a background session to ask about.
    let has_background = store.card_links(&card_id)?.background.is_some();
    let live = if has_background {
        crate::board::live_sessions(&store, &project_id)
    } else {
        Vec::new()
    };
    // The same rule the sidebar's poll applies, for this card's panes only.
    let mut leaves = crate::card_reconcile::card_leaves(&store, &project_id);
    leaves.retain(|_, pane| pane.card_id == card_id);
    if !leaves.is_empty() {
        let seq = crate::card_activity::next_seq();
        if let Ok(fronts) = crate::shell_launch::running_in(&project_id) {
            let registry = crate::card_activity::registry();
            crate::card_reconcile::reconcile_in(registry, &leaves, &fronts, seq);
        }
    }
    let heard = crate::card_activity::snapshot(&card_id);
    let sessions =
        crate::card_sessions::card_sessions(&store, &project_id, &card_id, &live, &heard);
    crate::card_activity::background_read(&card_id, &sessions);
    card.activity = crate::card_activity::activity(&sessions);

    let row = store
        .card(&card_id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such card"))?;

    let here = store
        .columns(&project_id)?
        .into_iter()
        .find(|column| column.id == card.column_id);
    let column_name = here
        .as_ref()
        .map(|column| column.name.clone())
        .unwrap_or_default();
    let column_step = here
        .and_then(|column| column.step_id)
        .and_then(|step_id| steps.iter().find(|one| one.id == step_id).cloned());

    Ok(CardDetail {
        runs: card.runs.clone(),
        comments: store.comments(&card_id)?.into_iter().map(spoken).collect(),
        pinned: store
            .attachments(&card_id)?
            .into_iter()
            .map(pinned_now)
            .collect(),
        worktree: checkout_of(&row),
        column_name,
        column_step,
        sessions,
        card,
    })
}

/// `card.set_due` — when this card is due, or nothing.
///
/// Seconds since the epoch, from the screen's own clock: a date is chosen in
/// the reader's timezone and this process has no business reinterpreting it.
#[tauri::command]
#[specta::specta]
pub fn card_set_due(
    project_id: String,
    card_id: String,
    due_at: Option<f64>,
) -> Result<CardDetail, RpcError> {
    let store = store()?;
    let at = match due_at {
        Some(seconds) if seconds.is_finite() && seconds > 0.0 => Some(seconds as i64),
        Some(_) => return Err(RpcError::new(ErrorCode::Invalid, "that is not a date")),
        None => None,
    };
    if !store.set_card_due(&card_id, at)? {
        return Err(RpcError::new(ErrorCode::NotFound, "no such card"));
    }
    detail_of(project_id, card_id)
}

/// What a comment body has to be before it is written.
pub(crate) fn sayable(body: &str) -> Result<&str, RpcError> {
    let said = body.trim();
    if said.is_empty() {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "an empty comment is not a comment",
        ));
    }
    // A ceiling before the row exists, so no read ever has to trust its own
    // table. An agent writing a comment has no natural end.
    if said.len() > limits::LONGEST_COMMENT {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            format!(
                "a comment is at most {} characters",
                limits::LONGEST_COMMENT
            ),
        ));
    }
    Ok(said)
}

/// `card.comment` — says something on the card.
#[tauri::command]
#[specta::specta]
pub fn card_comment(
    project_id: String,
    card_id: String,
    body: String,
) -> Result<CardDetail, RpcError> {
    let store = store()?;
    store.add_comment(&card_id, YOU, sayable(&body)?)?;
    detail_of(project_id, card_id)
}

/// `card.comment_edit` — changes one, and says that it changed.
#[tauri::command]
#[specta::specta]
pub fn card_comment_edit(
    project_id: String,
    card_id: String,
    comment_id: String,
    body: String,
) -> Result<CardDetail, RpcError> {
    let store = store()?;
    if !store.edit_comment(&comment_id, sayable(&body)?)? {
        return Err(RpcError::new(ErrorCode::NotFound, "no such comment"));
    }
    detail_of(project_id, card_id)
}

/// `card.comment_delete` — takes one out of the conversation.
#[tauri::command]
#[specta::specta]
pub fn card_comment_delete(
    project_id: String,
    card_id: String,
    comment_id: String,
) -> Result<CardDetail, RpcError> {
    let store = store()?;
    if !store.delete_comment(&comment_id)? {
        return Err(RpcError::new(ErrorCode::NotFound, "no such comment"));
    }
    detail_of(project_id, card_id)
}

/// `card.pin` — pins a file to the card.
///
/// The path is checked the same way `path.open` checks one, and for the same
/// reason: a pin is a path this app will later hand to the desktop, so a pin
/// outside the project is a pin that must not be made.
#[tauri::command]
#[specta::specta]
pub fn card_pin(
    project_id: String,
    card_id: String,
    path: String,
    label: Option<String>,
) -> Result<CardDetail, RpcError> {
    let store = store()?;
    let (_, root) = locate(&store, &project_id)?;
    let home = Store::root()?;
    let resolved = crate::reveal::openable(&[root], &home, Path::new(&path)).ok_or_else(|| {
        RpcError::new(
            ErrorCode::Forbidden,
            "that file is not in this project or in the devpit workspace",
        )
    })?;

    let named = label
        .map(|given| given.trim().to_owned())
        .filter(|given| !given.is_empty())
        .unwrap_or_else(|| {
            resolved
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| resolved.display().to_string())
        });
    if named.chars().count() > limits::LONGEST_LABEL {
        return Err(RpcError::new(ErrorCode::Invalid, "that label is too long"));
    }

    store.attach(&card_id, &resolved.display().to_string(), &named, None)?;
    detail_of(project_id, card_id)
}

/// `card.unpin` — unpins one. The file on disk is never touched.
#[tauri::command]
#[specta::specta]
pub fn card_unpin(
    project_id: String,
    card_id: String,
    pin_id: String,
) -> Result<CardDetail, RpcError> {
    let store = store()?;
    if !store.detach(&pin_id)? {
        return Err(RpcError::new(ErrorCode::NotFound, "no such attachment"));
    }
    detail_of(project_id, card_id)
}
