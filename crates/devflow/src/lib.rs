//! Devflow: a short engineering flow — briefing, context, plan, execution,
//! review, completion — that devpit serves through its MCP.
//!
//! A project configures it in `.devpit/devflow/devflow.yaml`, which is the
//! source of truth: nothing of it is copied into devpit's store. This crate
//! reads and writes that folder, detects what a setup starts from, writes a
//! brief and a plan as the text of the cards they become, runs a card's
//! checks into the evidence kept for it, and gates the card on that evidence. It knows nothing of the window: the
//! machine's half of detection (agent CLIs, the Decisions key), the board and
//! the person's click are the app's.

pub mod brief;
pub mod config;
pub mod detect;
pub mod evidence;
pub mod folder;
pub mod gate;
mod node;
pub mod plan;
pub mod playwright;
pub mod plugin;
pub mod preset;
pub mod proposal;
pub mod render;
pub mod runs;
pub mod setup;
pub mod skills;

pub use config::{Config, Invalid};
pub use folder::State;
pub use proposal::{Ask, Proposal};
