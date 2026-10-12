//! Devflow: a short engineering flow — briefing, context, plan, execution,
//! review, completion — that devpit serves through its MCP.
//!
//! A project configures it in `.devpit/devflow/devflow.yaml`, which is the
//! source of truth: nothing of it is copied into devpit's store. This crate
//! reads and writes that folder, and detects what a setup starts from. It
//! knows nothing of the window: the machine's half of detection (agent CLIs,
//! the Decisions key) and the person's click are the app's.

pub mod config;
pub mod detect;
pub mod folder;
mod node;
pub mod preset;
pub mod proposal;
pub mod render;
pub mod setup;

pub use config::{Config, Invalid};
pub use folder::State;
pub use proposal::{Ask, Proposal};
