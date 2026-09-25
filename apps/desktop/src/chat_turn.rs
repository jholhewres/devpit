//! What a chat turn settles before anything runs.
//!
//! Apart from `chat.rs`, which had grown to its ceiling holding the command and
//! every rule a turn follows at once.

use devpit_agentcli::head::Head;
use devpit_rpc::{ErrorCode, RpcError};

/// The head written before the turn runs.
///
/// What the conversation already settled carries over; a turn may still change
/// the model, the budget, the permission mode and the effort.
pub(crate) fn opening(
    head: Option<&Head>,
    profile_id: &str,
    model: Option<String>,
    budget_usd: Option<f64>,
    permission: Option<String>,
    effort: Option<String>,
    now: f64,
) -> Head {
    Head {
        profile: profile_id.to_owned(),
        model,
        created_at: head.map_or(now, |head| head.created_at),
        cost_usd: head.map(|head| head.cost_usd).unwrap_or_default(),
        budget_usd: budget_usd.or(head.and_then(|head| head.budget_usd)),
        session_id: head.and_then(|head| head.session_id.clone()),
        permission: permission
            .or_else(|| head.and_then(|head| head.permission.clone()))
            // Nothing to ask with yet: a turn left waiting on a prompt this
            // screen cannot draw would hang with no way to answer it.
            .or_else(|| Some("acceptEdits".to_owned())),
        effort: effort.or_else(|| head.and_then(|head| head.effort.clone())),
        title: head.and_then(|head| head.title.clone()),
        rewind: head.map(|head| head.rewind.clone()).unwrap_or_default(),
        cwd: head.and_then(|head| head.cwd.clone()),
        context: head.and_then(|head| head.context),
    }
}

/// Where a turn runs: the folder the conversation was fixed to, or the one the
/// window asked for when it was fixed to none.
///
/// Never a folder made for the turn. One that has gone is refused rather than
/// replaced: a session resumed anywhere else is a session the CLI does not find.
pub(crate) fn turn_cwd(fixed: Option<&str>, asked: &str) -> Result<String, RpcError> {
    match fixed {
        None => Ok(asked.to_owned()),
        Some(folder) if std::path::Path::new(folder).is_dir() => Ok(folder.to_owned()),
        Some(_) => Err(RpcError::new(
            ErrorCode::Conflict,
            "the folder this conversation ran in is gone",
        )),
    }
}

/// The profile a chat turn runs as, and the program to spawn for it: a name
/// only the shell knows is refused here, since a turn is spawned, not typed.
pub(crate) fn spawnable(profile_id: &str) -> Result<(devpit_rpc::Profile, String), RpcError> {
    let Some(profile) = crate::agent_profiles::all(&crate::projects::store()?)?
        .into_iter()
        .find(|found| found.id == profile_id)
    else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("no profile called {profile_id}"),
        ));
    };
    // Spawned, not typed, so a name only the shell knows is no use here — and
    // saying so beats "not on the PATH" about something the person watches
    // their terminal run every day.
    let Some(path) = profile.path.clone() else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            match profile.reach {
                devpit_rpc::Reach::ShellOnly => format!(
                    "{} is a shell function, so devpit cannot start it here — \
                     name the program it runs instead",
                    profile.command
                ),
                _ => format!("{} is not installed", profile.command),
            },
        ));
    };
    Ok((profile, path))
}

#[cfg(test)]
#[path = "chat_turn_tests.rs"]
mod tests;

/// The head after a turn, holding how full the context was when it ended. A
/// turn that reported none — stopped before its end frame — keeps the last
/// known reading rather than blanking the meter.
pub(crate) fn after(
    opening: Head,
    turn_id: &str,
    end: &devpit_rpc::TurnEnd,
    session_id: Option<String>,
    anchor: Option<String>,
) -> Head {
    let kept = opening.context;
    Head {
        context: end.context.or(kept),
        ..devpit_agentcli::head::after_turn(opening, turn_id, end.cost_usd, session_id, anchor)
    }
}
