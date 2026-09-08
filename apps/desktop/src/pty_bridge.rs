//! The binary bridge: pty frames to the webview without going through JSON.
//!
//! Tauri's channel serialises to JSON by default. At 5KB a payload that costs
//! 0.5–2ms in `JSON.parse`, and a verbose build produces those faster than the
//! frame budget allows. `InvokeResponseBody::Raw` skips the encoder entirely:
//! the bytes arrive as an ArrayBuffer and xterm.js takes them as-is.

use std::sync::atomic::Ordering;

use portable_pty::{CommandBuilder, PtySize};
use quockpit_rpc::{ErrorCode, RpcError};
use tauri::ipc::{Channel, InvokeResponseBody};

/// What the load test measured. Every field is counted, none estimated.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Throughput {
    pub bytes: u64,
    pub frames: u64,
    pub millis: u64,
    pub bytes_per_second: u64,
}

/// Runs a command on a pty and streams its output as binary frames.
///
/// Used by the load test, and the shape a real session command will take:
/// the channel is the transport either way.
#[tauri::command]
#[specta::specta]
pub async fn pty_drain(
    command: String,
    on_frame: Channel<InvokeResponseBody>,
) -> Result<Throughput, RpcError> {
    let mut builder = CommandBuilder::new("sh");
    builder.arg("-c");
    builder.arg(&command);

    let mut session = quockpit_pty::spawn(
        builder,
        PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        },
    )
    .map_err(|e| RpcError::new(ErrorCode::Internal, e.to_string()))?;

    let started = std::time::Instant::now();

    while let Some(frame) = session.frames.recv().await {
        // Raw, not Json: this is the whole point of the binary path.
        if on_frame.send(InvokeResponseBody::Raw(frame)).is_err() {
            // The window went away mid-flood. Not an error — it is what
            // closing a tab looks like from here.
            break;
        }
    }

    let millis = started.elapsed().as_millis() as u64;
    let bytes = session.counters.bytes.load(Ordering::Relaxed);

    Ok(Throughput {
        bytes,
        frames: session.counters.frames.load(Ordering::Relaxed),
        millis,
        bytes_per_second: if millis == 0 {
            0
        } else {
            bytes * 1000 / millis
        },
    })
}
