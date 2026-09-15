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
    let attach = devpit_agentcli::attach_argv(short_id).join(" ");
    Ok(format!("cd '{folder}' && {attach}"))
}

#[cfg(test)]
#[path = "attaching_tests.rs"]
mod tests;
