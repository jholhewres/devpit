//! How an agent reaches the devpit that is running: a CLI and an MCP server,
//! both in the app's own binary.
//!
//! `devpit agent …` and `devpit mcp` are answered here before any window is
//! made. Both post to the listener's `/agent` route through the same door the
//! hooks use — the endpoint and the secret read off disk on every call — so a
//! restarted app is found at its new port, and nothing is written into any
//! tool's own configuration to make this work.
//!
//! No dependency on the rest of the app: the caller hands over where devpit
//! keeps its state, and everything else is a socket and some JSON.

pub mod cli;
pub mod client;
pub mod guide;
pub mod mcp;

pub use client::ask;
