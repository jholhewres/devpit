//! The preference keys, in one place.
//!
//! Apart from `settings.rs` for the same reason `migrations_list.rs` is
//! apart from `migrations.rs`: that file holds how a preference is read and
//! written, and this one holds what they are called. A key spelled
//! differently in two files is a setting that silently forgets itself, and
//! nothing about the symptom points at the typo.

pub const ONBOARDED_AT: &str = "onboarding.completed_at";
pub const THEME: &str = "appearance.theme";
pub const AUTO_UPDATE: &str = "general.automatic_updates";
/// Whether closing a terminal with something running stops to ask.
pub const CONFIRM_STOP: &str = "terminal.confirm_stop";
/// The minimum contrast the terminal lifts colours to, as a number from 1 to 21.
pub const TERMINAL_CONTRAST: &str = "terminal.minimum_contrast";
/// Where new worktrees are created. Empty is the devpit workspace.
pub const WORKTREE_BASE: &str = "worktrees.base";
/// The apps offered in "Open in", as JSON.
pub const OPEN_IN_APPS: &str = "apps.open_in";
/// Which worktree origins are hidden from the lists, comma-separated.
pub const WORKTREES_HIDDEN: &str = "worktrees.hidden";
/// The agent profiles the person declared, as JSON.
pub const AGENT_PROFILES: &str = "agents.profiles";
/// What a new terminal opens, by agent or profile id. Empty is a shell.
pub const AGENT_DEFAULT: &str = "agents.default";
/// The ids kept out of the menus, as a JSON array.
pub const AGENTS_DISABLED: &str = "agents.disabled";
/// Whether devpit tells the agents it starts to report what they are doing.
pub const AGENT_HOOKS: &str = "agents.hooks";
/// How wide the two side panels were left, in pixels.
pub const SIDEBAR_WIDTH: &str = "layout.sidebar_width";
pub const FILES_WIDTH: &str = "layout.files_width";
/// The focus that is on, as `<project id>:<seconds since the epoch>`, or
/// absent. Two fields in one row because a focus is one thing: half of it
/// written and half not is a focus with no beginning.
pub const HEADS_DOWN: &str = "focus.heads_down";
/// Whether the focus mode is offered. Off unless it is turned on.
pub const FOCUS_MODE: &str = "focus.enabled";
/// Whether devpit keeps its own errors for a report. Off unless turned on.
pub const ERROR_REPORTS: &str = "telemetry.error_reports";
