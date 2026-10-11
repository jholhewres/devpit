//! What a tmux command can come back with instead of an answer.

#[derive(Debug, thiserror::Error)]
pub enum TmuxError {
    #[error("tmux is not installed, or not on PATH")]
    Missing,

    #[error("tmux {command} failed: {stderr}")]
    Failed { command: String, stderr: String },

    #[error("tmux {command} did not answer within {seconds}s")]
    Unanswered { command: String, seconds: u64 },
}
