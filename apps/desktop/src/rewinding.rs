//! Going back to an earlier turn, as a new conversation.
//!
//! Forked rather than rewound in place: the original keeps every turn it had,
//! on devpit's side and on the CLI's. The fork copies devpit's messages up to
//! the chosen turn, and its head asks the next turn to run with
//! `--fork-session --resume-session-at`, so the CLI brings the same history cut
//! at the same place.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use devpit_agentcli::head::{head_path, read_head, write_head, Anchor, Head, Rewind};
use devpit_agentcli::store::{append, conversation_path, read};
use devpit_rpc::{ErrorCode, Message, Part, Role, RpcError};

fn head_of(sessions: &Path, from: &str) -> Result<Head, RpcError> {
    read_head(&head_path(sessions, from))
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such conversation"))
}

/// Writes conversation `to` as `from` up to the end of `turn_id`.
pub(crate) fn rewind(
    sessions: &Path,
    from: &str,
    turn_id: &str,
    to: &str,
    now: f64,
) -> Result<(), RpcError> {
    let head = head_of(sessions, from)?;
    let Some(at) = head
        .rewind
        .anchors
        .iter()
        .position(|anchor| anchor.turn_id == turn_id)
    else {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "that turn ran before devpit kept its place in the CLI's transcript",
        ));
    };
    let (messages, _) = read(&conversation_path(sessions, from));
    let Some(last) = messages
        .iter()
        .rposition(|message| message.turn_id.as_deref() == Some(turn_id))
    else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that turn is not in the conversation",
        ));
    };
    let anchors = head.rewind.anchors[..=at].to_vec();
    fork(sessions, from, &messages[..=last], &head, anchors, to, now)
}

/// Writes conversation `to` as `from` before `turn_id` began, so its prompt
/// can be edited and sent again. Before the first turn, the CLI starts afresh.
pub(crate) fn rewind_before(
    sessions: &Path,
    from: &str,
    turn_id: &str,
    to: &str,
    now: f64,
) -> Result<(), RpcError> {
    let head = head_of(sessions, from)?;
    let (messages, _) = read(&conversation_path(sessions, from));
    let (first, anchors) = before_turn(&head, &messages, turn_id)?;
    fork(sessions, from, &messages[..first], &head, anchors, to, now)
}

/// Where `turn_id` begins, and the places of the turns before it.
fn before_turn(
    head: &Head,
    messages: &[Message],
    turn_id: &str,
) -> Result<(usize, Vec<Anchor>), RpcError> {
    let Some(first) = messages
        .iter()
        .position(|message| message.turn_id.as_deref() == Some(turn_id))
    else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that turn is not in the conversation",
        ));
    };
    let earlier: BTreeSet<&str> = messages[..first]
        .iter()
        .filter_map(|message| message.turn_id.as_deref())
        .collect();
    let anchors: Vec<Anchor> = head
        .rewind
        .anchors
        .iter()
        .take_while(|anchor| earlier.contains(anchor.turn_id.as_str()))
        .cloned()
        .collect();
    let answered = messages[..first]
        .iter()
        .any(|message| message.role == Role::Assistant);
    if anchors.is_empty() && answered {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "the turns before it ran before devpit kept their place in the CLI's transcript",
        ));
    }
    Ok((first, anchors))
}

/// `kept`, a note naming where it came from, and a head that resumes the CLI
/// at the last of `anchors`, or with none, starts it afresh.
fn fork(
    sessions: &Path,
    from: &str,
    kept: &[Message],
    head: &Head,
    anchors: Vec<Anchor>,
    to: &str,
    now: f64,
) -> Result<(), RpcError> {
    let file = conversation_path(sessions, to);
    if file.exists() {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "that conversation exists",
        ));
    }
    let internal = |err: std::io::Error| RpcError::internal(err.to_string());
    for message in kept {
        append(&file, message).map_err(internal)?;
    }
    let turn = kept
        .iter()
        .filter(|message| message.role == Role::User)
        .count() as u32;
    append(
        &file,
        &Message {
            id: format!("msg_{}", ulid::Ulid::generate()),
            turn_id: None,
            role: Role::System,
            parts: vec![Part::Rewound {
                from_conversation: from.to_owned(),
                turn,
            }],
            created_at: now,
            streaming: false,
            ended: None,
        },
    )
    .map_err(internal)?;

    let last = anchors.last().cloned();
    write_head(
        &head_path(sessions, to),
        &Head {
            created_at: now,
            session_id: last.as_ref().map(|anchor| anchor.session_id.clone()),
            rewind: Rewind {
                fork_at: last.map(|anchor| anchor.uuid),
                anchors,
            },
            // Cost and cap come along: a fork is not a way to spend a budget twice.
            ..head.clone()
        },
    )
    .map_err(internal)
}

/// What the turns in `messages` changed: the tree the first of them found,
/// and every path any of them touched. None when none changed a file.
fn changed_in(messages: &[Message]) -> Result<Option<(String, Vec<String>)>, RpcError> {
    let mut found: Option<Option<String>> = None;
    let mut paths = BTreeSet::new();
    for part in messages.iter().flat_map(|message| &message.parts) {
        if let Part::Changes { files, before } = part {
            found.get_or_insert_with(|| before.clone());
            paths.extend(files.iter().map(|file| file.path.clone()));
        }
    }
    match found {
        None => Ok(None),
        Some(None) => Err(RpcError::new(
            ErrorCode::Conflict,
            "those turns ran before devpit kept the files as they found them",
        )),
        Some(Some(tree)) => Ok(Some((tree, paths.into_iter().collect()))),
    }
}

/// Puts `checkout` back as `turn_id` found it, for every path it and the turns
/// after it changed. Checked as a fork first, so the disk never moves for an
/// edit that would then be refused.
pub(crate) fn put_back_from(
    sessions: &Path,
    checkout: &Path,
    from: &str,
    turn_id: &str,
) -> Result<(), RpcError> {
    let head = head_of(sessions, from)?;
    let (messages, _) = read(&conversation_path(sessions, from));
    let (first, _) = before_turn(&head, &messages, turn_id)?;
    let Some((tree, paths)) = changed_in(&messages[first..])? else {
        return Ok(());
    };
    for path in &paths {
        devpit_core::paths::resolve_new(checkout, path)
            .map_err(|err| RpcError::forbidden(err.to_string()))?;
    }
    devpit_git::put_back(checkout, &tree, &paths)
        .map_err(|err| RpcError::new(ErrorCode::Conflict, err.to_string()))
}

/// `chat.rewind` — a new conversation that goes on from an earlier turn.
///
/// Answers its id, which the window opens as a chat tab.
#[tauri::command]
#[specta::specta]
pub async fn chat_rewind(
    project_id: String,
    conversation_id: String,
    turn_id: String,
) -> Result<String, RpcError> {
    crate::off_main::blocking(move || chat_rewind_now(project_id, conversation_id, turn_id)).await
}

/// [`chat_rewind`], on the calling thread.
pub(crate) fn chat_rewind_now(
    project_id: String,
    conversation_id: String,
    turn_id: String,
) -> Result<String, RpcError> {
    use crate::adopting::plain;
    if !plain(&project_id) || !plain(&conversation_id) || !plain(&turn_id) {
        return Err(RpcError::new(ErrorCode::Forbidden, "that is not a turn"));
    }
    let to = format!("conv_{}", ulid::Ulid::generate());
    rewind(
        &crate::projects::project_home(&project_id)?.sessions(),
        &conversation_id,
        &turn_id,
        &to,
        now(),
    )?;
    Ok(to)
}

/// `chat.edit_from` — a new conversation from before a turn, with its prompt
/// to edit. With `put_back`, the files that turn and the later ones changed go
/// back first, as the turn found them.
#[tauri::command]
#[specta::specta]
pub async fn chat_edit_from(
    project_id: String,
    conversation_id: String,
    turn_id: String,
    put_back: bool,
) -> Result<String, RpcError> {
    crate::off_main::blocking(move || {
        chat_edit_from_now(project_id, conversation_id, turn_id, put_back)
    })
    .await
}

/// [`chat_edit_from`], on the calling thread.
pub(crate) fn chat_edit_from_now(
    project_id: String,
    conversation_id: String,
    turn_id: String,
    put_back: bool,
) -> Result<String, RpcError> {
    use crate::adopting::plain;
    if !plain(&project_id) || !plain(&conversation_id) || !plain(&turn_id) {
        return Err(RpcError::new(ErrorCode::Forbidden, "that is not a turn"));
    }
    let sessions = crate::projects::project_home(&project_id)?.sessions();
    if put_back {
        // Where its turns ran: the folder it was fixed to, else the project's.
        let fixed = read_head(&head_path(&sessions, &conversation_id)).and_then(|head| head.cwd);
        let checkout = match fixed {
            Some(cwd) => PathBuf::from(cwd),
            None => crate::projects::locate(&crate::projects::store()?, &project_id)?.1,
        };
        put_back_from(&sessions, &checkout, &conversation_id, &turn_id)?;
    }
    let to = format!("conv_{}", ulid::Ulid::generate());
    rewind_before(&sessions, &conversation_id, &turn_id, &to, now())?;
    Ok(to)
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as f64)
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "rewinding_tests.rs"]
mod tests;
