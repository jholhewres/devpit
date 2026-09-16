//! Reading a chat turn's stream, line by line.
//!
//! Apart from `talk.rs` because that file starts the process and owns its
//! stdin; this one only reads what the CLI wrote, and says what the stream
//! said about itself.

use devpit_rpc::{Part, SessionInit};

use crate::control::{may_close, Control, Running};
use crate::driver::{Driver, Read};

/// What a turn's stream said, apart from its parts.
#[derive(Default)]
pub(crate) struct Heard {
    /// The last id seen, not the first: `/clear` starts a new session inside
    /// the turn, and resuming the one it left would undo it.
    pub session_id: Option<String>,
    pub init: Option<SessionInit>,
    pub anchor: Option<String>,
    /// The CLI's own reason, cost and error flag. `None` when the stream ended
    /// without an end frame, which means the turn was stopped.
    pub ended: Option<(Option<String>, Option<f64>, bool)>,
}

/// Follows the stream, handing each part to `on_part` as it arrives, and
/// closing the turn's stdin as soon as the CLI can no longer be asked for
/// anything.
pub(crate) fn follow(
    driver: &dyn Driver,
    lines: impl Iterator<Item = String>,
    control: &Control,
    mut on_part: impl FnMut(Part),
) -> Heard {
    let mut heard = Heard::default();
    let mut running = Running::default();
    let mut result_seen = false;

    for line in lines {
        if let Some(seen) = driver.session(&line) {
            heard.session_id = Some(seen);
        }
        heard.anchor = driver.anchor(&line).or(heard.anchor);
        match driver.read(&line) {
            Read::Parts(parts) => parts.into_iter().for_each(|part| {
                running.saw(&part);
                on_part(part);
            }),
            Read::Ended {
                stop_reason,
                cost_usd,
                is_error,
            } => {
                heard.ended = Some((stop_reason, cost_usd, is_error));
                result_seen = true;
            }
            Read::Init(said) => heard.init = Some(said),
            Read::Nothing => {}
        }
        if may_close(result_seen, &running) {
            control.close();
        }
    }

    heard
}
