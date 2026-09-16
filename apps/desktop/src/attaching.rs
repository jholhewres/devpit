//! Where a card's background session is brought into a terminal.

use std::path::Path;

use devpit_rpc::{ErrorCode, RpcError};

use crate::shell_launch::Ready;

/// The line that brings a background session into the card's terminal, in the
/// card's checkout.
///
/// A pane already running something is refused rather than typed over: the
/// line would land in whatever program has the keyboard. The line moves to the
/// checkout first, because a card tab opened before checkouts existed sits at
/// the project root, where the card's work is not.
pub(crate) fn attach_target(
    checkout: &Path,
    short_id: &str,
    runner: Option<&devpit_agentcli::running::Runner>,
    ready: &Ready,
) -> Result<String, RpcError> {
    if let Ready::Busy(command) = ready {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            format!("this card's terminal is running {command}"),
        ));
    }
    // Single quotes cannot hold a single quote, and a path that needs one is
    // not a path to type into a shell.
    let folder = checkout.display().to_string();
    if folder.contains('\'') || !crate::adopting::plain(short_id) {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "this card's checkout cannot be typed into a terminal",
        ));
    }
    // The profile's binary, and its variables in front of it — the same line
    // the launch builds. Typed into a shell, so a token in one of those
    // variables is in that terminal's scrollback, exactly as it already is on
    // launch (`running::line`). Declared, not solved: the keyring is plan 17.
    let attach = devpit_agentcli::attach_argv(runner, short_id).join(" ");
    let env = runner
        .map(devpit_agentcli::running::assignments)
        .unwrap_or_default();
    Ok(format!("cd '{folder}' && {env}{attach}"))
}

#[cfg(test)]
#[path = "attaching_tests.rs"]
mod tests;
