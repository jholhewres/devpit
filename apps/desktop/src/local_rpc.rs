//! `POST /rpc/<command>`: a browser's call, handed to the main webview's own
//! invoke handler as the window's IPC would hand it.
//!
//! The request is built here, in Rust, with the app's own URL and the invoke
//! key the app was started with. Neither comes from the browser and neither
//! goes back to it: the answer is the command's body, or its error, alone.
//! The webview does not need to be shown or focused: the handler runs on the
//! app's side and only uses the webview as the handle it was given.

use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc;
use std::time::Duration;

use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponse, InvokeResponseBody};
use tauri::webview::InvokeRequest;
use tauri::Manager;

use crate::remote_http::reply;

/// A command's longest wait before the browser is told it timed out.
const WAITS: Duration = Duration::from_secs(120);

/// The URL the call is made from: the app's own, so the handler treats it as
/// the window's. Tauri's local origin differs by platform.
fn app_url() -> tauri::Url {
    let url = if cfg!(any(windows, target_os = "android")) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    };
    url.parse().expect("a fixed URL")
}

/// A command name the handler could know: letters, digits and `_`, nothing that
/// reaches a plugin or a path.
pub(crate) fn plain_command(command: &str) -> bool {
    !command.is_empty()
        && command.len() <= 128
        && command
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// What the browser is answered, from what the handler answered.
pub(crate) fn answered(response: InvokeResponse) -> (&'static str, Vec<u8>) {
    match response {
        InvokeResponse::Ok(InvokeResponseBody::Json(json)) => ("200 OK", json.into_bytes()),
        InvokeResponse::Ok(InvokeResponseBody::Raw(bytes)) => ("200 OK", bytes),
        InvokeResponse::Err(error) => (
            "500 Internal Server Error",
            error.0.to_string().into_bytes(),
        ),
    }
}

pub(crate) fn call(app: &tauri::AppHandle, stream: &mut TcpStream, command: &str, body: &[u8]) {
    if !plain_command(command) {
        reply(stream, "404 Not Found", "text/plain", b"no such command");
        return;
    }
    if crate::local_server::desktop_only(command) {
        reply(
            stream,
            "501 Not Implemented",
            "text/plain",
            b"only in the desktop app",
        );
        return;
    }
    let args = if body.is_empty() {
        serde_json::Value::Object(Default::default())
    } else {
        match serde_json::from_slice(body) {
            Ok(args) => args,
            Err(_) => {
                reply(
                    stream,
                    "400 Bad Request",
                    "text/plain",
                    b"the body is not JSON",
                );
                return;
            }
        }
    };
    let Some(webview) = app.get_webview("main") else {
        reply(
            stream,
            "503 Service Unavailable",
            "text/plain",
            b"devpit has no window yet",
        );
        return;
    };
    let (sent, got) = mpsc::sync_channel(1);
    webview.on_message(
        InvokeRequest {
            cmd: command.to_owned(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: app_url(),
            body: InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: app.invoke_key().to_owned(),
        },
        Box::new(move |_webview, _command, response, _callback, _error| {
            let _ = sent.send(response);
        }),
    );
    match got.recv_timeout(WAITS) {
        Ok(response) => {
            let (status, body) = answered(response);
            reply(stream, status, "application/json", &body);
        }
        Err(_) => reply(
            stream,
            "504 Gateway Timeout",
            "text/plain",
            b"the command did not answer",
        ),
    }
    let _ = stream.flush();
}

#[cfg(test)]
#[path = "local_rpc_tests.rs"]
mod tests;
