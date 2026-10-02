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
/// The keys that bring devpit forward from anywhere, as the shortcut
/// plugin spells them (`CommandOrControl+Shift+D`). Absent is none.
pub const SHORTCUT: &str = "general.shortcut";
/// The moment a pause ends, in seconds since the epoch; `0` is none.
pub const PAUSED_UNTIL: &str = "focus.paused_until";
/// Whether a card's date goes off as a reminder at its time. On unless
/// turned off.
pub const REMINDERS: &str = "general.reminders";
/// Whether the island opens above the other windows. On unless turned off.
pub const ISLAND: &str = "island.enabled";
/// The screen the island was last dragged to, by the name the system gives it.
pub const ISLAND_SCREEN: &str = "island.screen";
/// Whether devpit keeps its own errors for a report. Off unless turned on.
pub const ERROR_REPORTS: &str = "telemetry.error_reports";
/// How a voice message becomes words: `local` (a whisper on this machine),
/// `api` (a transcription service the person chose) or `off`. Local unless set.
pub const TRANSCRIBE: &str = "chat.transcribe";
/// The model: a whisper.cpp `.bin` file, a whisper model's name, or the
/// service's model. Empty is the engine's default.
pub const TRANSCRIBE_MODEL: &str = "chat.transcribe_model";
/// The language a voice message is heard in, as a code; empty is the system's.
pub const TRANSCRIBE_LANGUAGE: &str = "chat.transcribe_language";
/// The transcription service's address, OpenAI's form of it. Empty is OpenAI.
pub const TRANSCRIBE_URL: &str = "chat.transcribe_url";
/// Whether this machine can be reached from the person's other devices,
/// over their tailnet. Off unless turned on.
pub const REMOTE: &str = "remote.enabled";
/// The loopback port the remote viewer is served from, kept so the tailnet
/// address stays the same across restarts.
pub const REMOTE_PORT: &str = "remote.port";
