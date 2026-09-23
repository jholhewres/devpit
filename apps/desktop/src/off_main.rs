//! Work that waits, kept off the thread the window draws on.
//!
//! A command that is a plain `fn` runs on Tauri's main thread — the one that
//! also paints the window and hands it every click. One that starts a process
//! (git, tmux, the person's own shell) holds that thread until the process
//! answers, and on a busy machine that is seconds: the window stops drawing
//! and stops taking input, and looks hung, because for that long it is.
//!
//! An `async` command runs on the async runtime instead, and anything blocking
//! inside it goes to the blocking pool, here.

use devpit_rpc::RpcError;

/// Runs `work` on the blocking pool and waits for it without holding a thread
/// the window needs.
pub(crate) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, RpcError> + Send + 'static,
) -> Result<T, RpcError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|err| RpcError::internal(err.to_string()))?
}
