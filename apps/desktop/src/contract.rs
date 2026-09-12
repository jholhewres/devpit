//! The command list, and the TypeScript generated from it.
//!
//! Separate from the window: which commands exist is not a question about how
//! the window is drawn, and keeping the list here means adding one does not
//! touch the file that opens the app.

use tauri_specta::{collect_commands, Builder};

use crate::{
    account, arranging, asking, board, branches, card_work, cards, chat, columns, commands, diffs,
    files, filetree, front, happening, history, in_flight, index, mcp, moving, notices, openers,
    panes, paths, priming, projects, reveal, saves, search, sessions, settings, shell_launch,
    sources, staging, steps, threads, workspace, worktree_base, worktrees,
};

/// Where the generated TypeScript lands.
///
/// Anchored to the manifest directory rather than the working directory:
/// `tauri dev` and a bare `./devpit-desktop` run from different places, and a
/// relative path would quietly write the contract somewhere else.
pub const BINDINGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/src/gen/bindings.ts");

/// The commands whose types are generated into TypeScript.
///
/// Separate from the invoke handler on purpose: one command cannot be in the
/// contract, because specta cannot describe the channel it streams over.
pub fn contract() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        commands::app_info,
        commands::app_health,
        commands::app_capabilities,
        projects::project_list,
        projects::project_add,
        projects::project_clone,
        projects::project_open,
        projects::project_forget,
        projects::project_rename,
        chat::chat_history,
        threads::chat_list,
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
        openers::apps_list,
        openers::apps_known,
        openers::apps_add,
        openers::apps_remove,
        openers::apps_open,
        worktrees::worktree_list,
        worktrees::worktree_remove,
        priming::worktree_prime_read,
        priming::worktree_prime_write,
        worktree_base::worktree_base_read,
        worktree_base::worktree_base_write,
        sources::worktree_sources,
        sources::worktree_source_show,
        filetree::project_tree,
        index::project_files,
        projects::project_changes,
        history::project_history,
        search::project_search,
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
        columns::column_set_flow,
        board::card_create,
        board::card_update,
        moving::card_move,
        cards::card_detail,
        cards::card_set_due,
        cards::card_comment,
        cards::card_comment_edit,
        cards::card_comment_delete,
        cards::card_pin,
        cards::card_unpin,
        cards::card_board,
        card_work::card_checkout,
        card_work::card_terminal,
        card_work::card_play,
        notices::notices_read,
        notices::notices_mark,
        notices::notices_mark_all,
        notices::notices_sweep_due,
        files::file_read,
        paths::path_create,
        paths::path_move,
        paths::path_delete,
        staging::changes_stage,
        staging::changes_unstage,
        staging::changes_commit,
        staging::changes_discard,
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
        arranging::session_split,
        arranging::session_close_leaf,
        arranging::session_close_tab,
        arranging::session_rename_leaf,
        arranging::session_set_ratio,
        panes::session_write,
        panes::session_resize,
        panes::pane_scrollback,
        panes::session_detach,
        shell_launch::session_running,
        shell_launch::agents_known,
        shell_launch::session_launch_agent,
        happening::terminal_happenings,
        account::account_read,
        account::account_sign_in,
        account::account_poll,
        account::account_sign_out,
        settings::settings_read,
        settings::settings_write,
        settings::settings_finish_onboarding,
    ])
}

#[cfg(test)]
#[path = "contract_tests.rs"]
mod tests;
