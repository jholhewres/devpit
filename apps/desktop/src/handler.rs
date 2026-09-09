//! Every command the window may call, at runtime.
//!
//! Separate from the contract on purpose, and the difference is the point:
//! one command cannot be in the *contract*, because specta cannot describe
//! the channel it streams over — and leaving it out of the *handler* would
//! mean the screen cannot call it at all. This list is what exists; that one
//! is what is typed.

use crate::asking;
use crate::branches;
use crate::chat;
use crate::index;
use crate::mcp;
use crate::pty_bridge;
use crate::reveal;
use crate::saves;
use crate::staging;
use crate::workspace;
use crate::worktrees;
use crate::{
    board, columns, commands, diffs, files, front, in_flight, panes, projects, sessions, settings,
    steps,
};

pub fn handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        commands::app_info,
        commands::app_health,
        commands::app_capabilities,
        projects::project_list,
        projects::project_add,
        projects::project_clone,
        projects::project_open,
        projects::project_forget,
        chat::chat_history,
        chat::chat_send,
        chat::chat_cancel,
        chat::chat_frames,
        chat::agent_profiles,
        chat::chat_attach,
        asking::permission_answer,
        asking::permission_ask_from_now,
        asking::permission_questions,
        mcp::mcp_list,
        workspace::skills_list,
        workspace::workspace_read,
        workspace::usage_read,
        reveal::path_open,
        reveal::path_reveal,
        worktrees::worktree_list,
        worktrees::worktree_remove,
        worktrees::worktree_prime_read,
        worktrees::worktree_prime_write,
        projects::project_tree,
        index::project_files,
        projects::project_changes,
        projects::project_history,
        branches::branch_list,
        branches::branch_switch,
        projects::project_notes,
        projects::project_note_add,
        board::board_get,
        columns::column_create,
        columns::column_rename,
        columns::column_reorder,
        columns::column_delete,
        columns::column_set_step,
        board::card_create,
        board::card_update,
        board::card_move,
        files::file_read,
        staging::changes_stage,
        staging::changes_unstage,
        staging::changes_commit,
        saves::file_write,
        diffs::file_diff,
        diffs::commit_diff,
        in_flight::run_cancel,
        front::card_archive,
        front::card_diff,
        columns::step_create,
        steps::agents_list,
        sessions::terminal_attach_agent,
        sessions::session_ensure,
        sessions::session_layout,
        sessions::session_focus,
        sessions::session_split,
        sessions::session_close_leaf,
        sessions::session_rename_leaf,
        sessions::session_set_ratio,
        panes::session_write,
        panes::session_resize,
        panes::pane_scrollback,
        panes::session_detach,
        panes::session_attach,
        settings::settings_read,
        settings::settings_write,
        settings::settings_finish_onboarding,
        pty_bridge::pty_drain,
    ]
}
