//! One live pane: the bytes in, the bytes out, and the size they are drawn at.
//!
//! [`crate::sessions`] owns the shape the panes are arranged in. This owns a
//! single one while something is looking at it — which is why everything here
//! takes a `client_id`: two windows onto the same pane is a race, and the
//! newer mount wins it on purpose.
//!
//! The commands here are still called `session.*` rather than `pane.*`. The
//! grouping this file makes is the right one and the names should follow it,
//! but renaming a command breaks every call site in `web/`, which is not this
//! front's to break. It is written down in `06-o-que-o-layout-pediu.md` for
//! the step that joins the two.

use std::io::Write;
use std::sync::{Arc, Mutex};

use devpit_rpc::{ErrorCode, PaneSize, RpcError};
use portable_pty::{CommandBuilder, PtySize};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{Emitter, State};

use crate::claims::Live;
use crate::happening::said;
use crate::sessions::SessionState;

/// `session.write` — bytes into the attached client of a leaf.
///
/// The count comes back rather than a bare unit: a keystroke that went nowhere
/// and a keystroke that landed look identical from the screen otherwise, and
/// the first is the one worth telling someone about.
#[tauri::command]
#[specta::specta]
pub fn session_write(
    state: State<SessionState>,
    pane_id: String,
    data: String,
) -> Result<PaneWritten, RpcError> {
    let live = state.claims.live(&pane_id)?;
    let mut writer = live
        .writer
        .lock()
        .map_err(|_| RpcError::internal("writer lock"))?;
    let bytes = data.len() as u32;
    writer
        .write_all(data.as_bytes())
        .and_then(|_| writer.flush())
        .map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(PaneWritten { bytes })
}

/// Response of `session.write`.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PaneWritten {
    pub bytes: u32,
}

/// `session.resize` — the pty size of an attached leaf, and what it became.
///
/// Reports the size the pty is at afterwards rather than the one asked for.
/// A pane that believes it has two hundred columns when it has eighty draws
/// wrongly, and the cause is a long way from the symptom.
#[tauri::command]
#[specta::specta]
pub fn session_resize(
    state: State<SessionState>,
    pane_id: String,
    rows: u16,
    cols: u16,
) -> Result<PaneSize, RpcError> {
    let live = state.claims.live(&pane_id)?;
    let master = live
        .master
        .lock()
        .map_err(|_| RpcError::internal("master lock"))?;
    master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| RpcError::internal(err.to_string()))?;

    // Asked back rather than echoed: the kernel is what decides, and it is
    // the only thing that can be asked honestly.
    let applied = master
        .get_size()
        .map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(PaneSize {
        rows: applied.rows,
        cols: applied.cols,
    })
}

/// `pane.scrollback` — what this pane has printed, oldest kept byte first.
///
/// This is what makes reopening a window show a terminal rather than an empty
/// one. It comes from the ring the reader fills, so it survives the webview
/// going away and dying with it — the pty never stopped.
#[tauri::command]
#[specta::specta]
pub fn pane_scrollback(
    state: State<SessionState>,
    pane_id: String,
) -> Result<PaneScrollback, RpcError> {
    // A pane nobody has attached yet has no history — which is an answer, not
    // a failure. Refusing here made opening a terminal for the first time
    // reject before it was ever attached, and the caller had no output and no
    // error to explain it.
    let Ok(live) = state.claims.live(&pane_id) else {
        return Ok(PaneScrollback {
            text: String::new(),
            truncated: false,
        });
    };
    let ring = live
        .ring
        .lock()
        .map_err(|_| RpcError::internal("ring lock"))?;
    let bytes = ring.replay();
    Ok(PaneScrollback {
        // Lossy rather than strict: the ring keeps bytes, and a pane that
        // printed something undecodable must still hand back the rest of its
        // history instead of nothing.
        text: String::from_utf8_lossy(&bytes).into_owned(),
        // The buffer drops the oldest once it is full, so this says whether
        // what came back is the whole history or only its tail.
        truncated: ring.len() == devpit_pty::RING_BYTES,
    })
}

/// Response of `pane.scrollback`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PaneScrollback {
    pub text: String,
    /// Older output has been dropped to make room.
    pub truncated: bool,
}

/// `session.detach` — closes only this app's client; tmux keeps the shell.
#[tauri::command]
#[specta::specta]
pub fn session_detach(
    state: State<SessionState>,
    pane_id: String,
    client_id: String,
) -> Result<(), RpcError> {
    if !state.claims.release(&pane_id, &client_id)? {
        return Ok(());
    }
    if let Some(live) = state.claims.take_if_theirs(&pane_id, &client_id)? {
        live.stop()?;
    }
    Ok(())
}

/// Attaches a client pty to the tmux window for this leaf and streams frames.
///
/// Not in the generated contract: the binary channel cannot be described by
/// specta, so its wrapper is hand-written, next to the xterm host.
#[tauri::command]
pub async fn session_attach(
    state: State<'_, SessionState>,
    project_id: String,
    pane_id: String,
    client_id: String,
    rows: u16,
    cols: u16,
    on_frame: Channel<InvokeResponseBody>,
) -> Result<(), RpcError> {
    // Which tab draws it does not matter here; that it is this project's does.
    crate::sessions::holding(&project_id, &pane_id)?;

    let argv = crate::sessions::attach_argv(&project_id, &pane_id)?;
    let mut builder = CommandBuilder::new(&argv[0]);
    for arg in &argv[1..] {
        builder.arg(arg);
    }
    builder.env("TERM", "xterm-256color");

    let claimed = state.claims.take_for(&pane_id, &client_id)?;
    if let Some(previous) = claimed.replaced {
        let _ = previous.stop();
    }

    let mut session = match devpit_pty::spawn(
        builder,
        PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        },
    ) {
        Ok(session) => session,
        Err(err) => {
            state.claims.release(&pane_id, &client_id)?;
            return Err(RpcError::internal(err.to_string()));
        }
    };

    // What the terminal says about itself, relayed while it says it.
    //
    // Its own task, not the frame loop: a title arriving between two frames of
    // a flood would otherwise wait for the flood to pause. The task ends when
    // the reader thread drops its sender, which is when the pty is gone.
    let mut told = session
        .take_told()
        .ok_or_else(|| RpcError::internal("the pty did not expose what it hears"))?;
    let telling = state.app().clone();
    let telling_pane = pane_id.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(one) = told.recv().await {
            let _ = telling.emit("terminal:happening", said(&telling_pane, one));
        }
    });

    let ring = Arc::clone(&session.ring);
    let Some(io) = session.take_io() else {
        state.claims.release(&pane_id, &client_id)?;
        return Err(RpcError::internal("the pty did not expose its io handles"));
    };
    let live = Arc::new(Live {
        client_id: client_id.clone(),
        writer: Mutex::new(io.writer),
        master: Mutex::new(io.master),
        killer: Mutex::new(io.killer),
        ring,
        pid: io.pid,
    });
    if !state
        .claims
        .install(&pane_id, claimed.generation, Arc::clone(&live))?
    {
        let _ = live
            .killer
            .lock()
            .map_err(|_| RpcError::internal("killer lock"))?
            .kill();
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "a newer client already owns that pane",
        ));
    }

    while let Some(frame) = session.frames.recv().await {
        if on_frame.send(InvokeResponseBody::Raw(frame)).is_err() {
            break;
        }
    }

    state.claims.forget_exactly(&pane_id, &live);
    let _ = state.claims.release(&pane_id, &client_id);
    Ok(())
}
