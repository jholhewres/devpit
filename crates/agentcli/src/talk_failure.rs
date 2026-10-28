//! What a CLI said on stderr, read apart from its answer: a pipe nobody
//! drains stops the CLI once it is full, and when the CLI stops without an
//! answer (a session it cannot resume, say), that is the only word on why.

use std::io::Read;
use std::process::Child;
use std::thread::JoinHandle;

use devpit_rpc::Part;

use crate::talk_stream::Ending;

/// The most of the CLI's stderr kept: the end, where the reason is.
const KEPT: usize = 4 * 1024;

/// Starts reading the child's stderr to its end, on a thread of its own.
pub(crate) fn read_apart(child: &mut Child) -> Option<JoinHandle<String>> {
    let pipe = child.stderr.take()?;
    Some(std::thread::spawn(move || tail_of(pipe)))
}

/// How a turn ended: its end frame, else stopped. A CLI that failed
/// without one has what it said on stderr told to the answer.
pub(crate) fn ending(
    ended: Option<Ending>,
    succeeded: bool,
    read: Option<JoinHandle<String>>,
    on_part: &mut impl FnMut(Part),
) -> Ending {
    let said = read.and_then(|read| read.join().ok()).unwrap_or_default();
    if ended.is_none() && !succeeded && !said.trim().is_empty() {
        on_part(Part::Text {
            text: format!("The agent stopped: {}", said.trim()),
            parent: None,
        });
    }
    // No end frame means the turn was stopped, not that it finished. A
    // conversation that shows those the same way is lying about one.
    ended.unwrap_or(Ending {
        stop_reason: Some("interrupted".to_owned()),
        cost_usd: None,
        is_error: !succeeded,
        context: None,
    })
}

/// The end of what `pipe` says, read to its close however long it is.
fn tail_of(mut pipe: impl Read) -> String {
    let mut kept: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    while let Ok(read) = pipe.read(&mut chunk) {
        if read == 0 {
            break;
        }
        kept.extend_from_slice(&chunk[..read]);
        if kept.len() > KEPT {
            kept.drain(..kept.len() - KEPT);
        }
    }
    String::from_utf8_lossy(&kept).into_owned()
}
