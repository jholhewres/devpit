//! Each pane's commands, kept as blocks while the app runs.
//!
//! The tap reads a pane's raw stream (`tap.rs`); here that stream is cut into
//! blocks (`devpit_pty::blocks`) and the finished ones are kept, bounded, so
//! the terminal can draw a command with its output and act on it — copy it,
//! run it again — long after it scrolled out of tmux's screen.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use devpit_pty::blocks::{Block, Cut, Head, History, Segmenter};
use devpit_rpc::{BlockChanged, CommandBlock, ErrorCode, FolderGlance, PaneBlocks, RpcError};
use tauri::{Emitter, State};

/// One pane's cutting and keeping.
#[derive(Default)]
struct Pane {
    segmenter: Segmenter,
    history: History,
}

/// Every pane's blocks, across projects.
#[derive(Default)]
pub struct Blocks {
    panes: Mutex<HashMap<String, Arc<Mutex<Pane>>>>,
    /// Which project each pane is in, for the bell.
    projects: Mutex<HashMap<String, String>>,
}

/// A command this long that ends while the window is elsewhere rings the
/// bell: long enough that you went to do something else while it ran.
const WORTH_TELLING_MS: u64 = 10_000;

impl Blocks {
    /// Says which project a pane belongs to; the tap knows when it arms one.
    pub(crate) fn belongs(&self, pane_id: &str, project_id: &str) {
        let mut projects = self
            .projects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        projects.insert(pane_id.to_owned(), project_id.to_owned());
    }

    fn pane(&self, pane_id: &str) -> Arc<Mutex<Pane>> {
        let mut panes = self
            .panes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Arc::clone(panes.entry(pane_id.to_owned()).or_default())
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or_default()
}

fn view(head: &Head) -> CommandBlock {
    CommandBlock {
        id: head.id as f64,
        command: head.command.clone(),
        cwd: head.cwd.clone(),
        started_at: head.started_at as f64,
        ended_at: head.ended_at.map(|at| at as f64),
        code: head.code,
        interactive: head.interactive,
        truncated: head.truncated,
    }
}

/// A chunk of a pane's raw stream: what it says about itself goes to the
/// window as it always did, and what it ran becomes blocks.
pub(crate) fn heard(app: &tauri::AppHandle, pane_id: &str, chunk: &[u8]) {
    let Some(blocks) = tauri::Manager::try_state::<Blocks>(app) else {
        return;
    };
    let pane = blocks.pane(pane_id);
    let mut pane = pane.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Pane { segmenter, history } = &mut *pane;
    let mut ended: Vec<Block> = Vec::new();
    segmenter.feed(
        chunk,
        now_ms(),
        |told| crate::tap::report(app, pane_id, told),
        |cut, block| {
            let head = match &cut {
                Cut::Started(head) | Cut::Changed(head) | Cut::Ended(head) => head,
            };
            let _ = app.emit(
                "terminal:block",
                BlockChanged {
                    pane_id: pane_id.to_owned(),
                    block: view(head),
                },
            );
            ended.extend(block);
        },
    );
    for block in ended {
        told_if_long(app, &blocks, pane_id, &block.head);
        history.push(block);
    }
}

/// Rings the bell for a long command that ended while the window was not in
/// front — which is when a finished build would otherwise go unnoticed.
fn told_if_long(app: &tauri::AppHandle, blocks: &Blocks, pane_id: &str, head: &Head) {
    let Some(ended) = head.ended_at else { return };
    if ended.saturating_sub(head.started_at) < WORTH_TELLING_MS || head.interactive {
        return;
    }
    let focused =
        tauri::Manager::get_webview_window(app, "main").and_then(|window| window.is_focused().ok());
    if focused != Some(false) {
        return;
    }
    let line = head.command.as_deref().unwrap_or("A command");
    let short: String = line.chars().take(60).collect();
    let title = match head.code {
        Some(0) => format!("{short} finished"),
        Some(code) => format!("{short} failed (exit {code})"),
        None => format!("{short} ended"),
    };
    let project = blocks
        .projects
        .lock()
        .ok()
        .and_then(|projects| projects.get(pane_id).cloned());
    crate::notices::ring(
        app,
        project.as_deref(),
        crate::notices::kind::COMMAND,
        &title,
        head.cwd.as_deref(),
        None,
    );
}

/// `pane.blocks` — every block this pane has kept, oldest first and the one
/// still running last, with where its shell stands.
#[tauri::command]
#[specta::specta]
pub fn pane_blocks(state: State<'_, Blocks>, pane_id: String) -> Result<PaneBlocks, RpcError> {
    let pane = state.pane(&pane_id);
    let pane = pane.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut blocks: Vec<CommandBlock> = pane.history.heads().iter().map(view).collect();
    if let Some(running) = pane.segmenter.running() {
        blocks.push(view(&running.head));
    }
    Ok(PaneBlocks {
        blocks,
        integrated: pane.segmenter.integrated(),
        at_prompt: pane.segmenter.at_prompt(),
        cwd: pane.segmenter.cwd().map(str::to_owned),
    })
}

/// `block.output` — what a block printed, as the terminal received it.
#[tauri::command]
#[specta::specta]
pub fn block_output(
    state: State<'_, Blocks>,
    pane_id: String,
    block_id: f64,
) -> Result<String, RpcError> {
    let pane = state.pane(&pane_id);
    let pane = pane.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let id = block_id as u64;
    let block = pane
        .history
        .get(id)
        .or_else(|| pane.segmenter.running().filter(|one| one.head.id == id))
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that block is no longer kept"))?;
    Ok(String::from_utf8_lossy(&block.output).into_owned())
}

/// `pane.blocks_clear` — forgets a pane's finished blocks.
#[tauri::command]
#[specta::specta]
pub fn pane_blocks_clear(state: State<'_, Blocks>, pane_id: String) -> Result<(), RpcError> {
    let pane = state.pane(&pane_id);
    pane.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .history
        .clear();
    Ok(())
}

/// `terminal.block_changes` — the shape `terminal:block` carries, so the
/// generated contract knows it (the reason `terminal.happenings` exists).
#[tauri::command]
#[specta::specta]
pub fn terminal_block_changes() -> Result<Vec<BlockChanged>, RpcError> {
    Ok(Vec::new())
}

/// `pane.submit` — a line typed in the terminal's own input, run in the pane.
///
/// Whatever the shell's line already holds is cleared first — the end of it,
/// then all of it back to the prompt, which is `C-e C-u` in bash, zsh and
/// fish alike — and the screen too, so the running command starts on a clean
/// one. Several lines go as a bracketed paste, so the shell takes them as one
/// command rather than running each as it arrives.
#[tauri::command]
#[specta::specta]
pub fn pane_submit(project_id: String, pane_id: String, line: String) -> Result<(), RpcError> {
    crate::sessions::holding(&project_id, &pane_id)?;
    let session = devpit_tmux::Server::session_name(&project_id);
    crate::sessions::tmux_server()?
        .submit(&devpit_tmux::Server::target(&session, &pane_id), &line)
        .map_err(|err| RpcError::internal(err.to_string()))
}

/// `folder.glance` — a folder's branch and its difference from `HEAD`, for
/// the terminal's prompt chips. Nothing outside a repository.
#[tauri::command]
#[specta::specta]
pub fn folder_glance(folder: String) -> Result<Option<FolderGlance>, RpcError> {
    let found = devpit_git::glance(std::path::Path::new(&folder))
        .map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(found.map(|one| FolderGlance {
        branch: one.branch,
        files: one.files,
        added: one.added,
        removed: one.removed,
    }))
}

/// `pane.compose` — a message written in devpit's own editor, handed to the
/// program in the pane (an agent's TUI) as one paste and sent.
#[tauri::command]
#[specta::specta]
pub fn pane_compose(project_id: String, pane_id: String, text: String) -> Result<(), RpcError> {
    crate::sessions::holding(&project_id, &pane_id)?;
    let session = devpit_tmux::Server::session_name(&project_id);
    crate::sessions::tmux_server()?
        .paste_and_send(&devpit_tmux::Server::target(&session, &pane_id), &text)
        .map_err(|err| RpcError::internal(err.to_string()))
}

/// How many completions a Tab offers. Past this the word needs more letters.
const MOST_COMPLETIONS: usize = 60;

/// `folder.complete` — what the last word typed in a terminal's editor could
/// finish as: the entries of the folder it names, relative to where the shell
/// is, `~` read as home. A folder ends in `/` so the next Tab goes into it.
/// Hidden entries only when the word asks for them with a leading dot.
#[tauri::command]
#[specta::specta]
pub fn folder_complete(cwd: String, word: String) -> Result<Vec<String>, RpcError> {
    Ok(completions(
        std::path::Path::new(&cwd),
        &word,
        std::env::var("HOME").ok().as_deref(),
    ))
}

pub(crate) fn completions(cwd: &std::path::Path, word: &str, home: Option<&str>) -> Vec<String> {
    let (dir, stem) = match word.rfind('/') {
        Some(at) => (&word[..=at], &word[at + 1..]),
        None => ("", word),
    };
    let base = if let Some(rest) =
        dir.strip_prefix("~/")
            .or(if dir == "~/" { Some("") } else { None })
    {
        match home {
            Some(home) => std::path::Path::new(home).join(rest),
            None => return Vec::new(),
        }
    } else if dir.starts_with('/') {
        std::path::PathBuf::from(dir)
    } else {
        cwd.join(dir)
    };
    let Ok(entries) = std::fs::read_dir(&base) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(stem) || (name.starts_with('.') && !stem.starts_with('.')) {
                return None;
            }
            let folder = entry.path().is_dir();
            Some(format!("{dir}{name}{}", if folder { "/" } else { "" }))
        })
        .collect();
    found.sort_by_key(|one| one.to_lowercase());
    found.truncate(MOST_COMPLETIONS);
    found
}

#[cfg(test)]
mod completion_tests {
    use super::completions;

    #[test]
    fn a_word_finishes_as_the_entries_of_the_folder_it_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "").unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        std::fs::write(dir.path().join(".env"), "").unwrap();
        assert_eq!(completions(dir.path(), "s", None), ["src/"]);
        assert_eq!(completions(dir.path(), "src/m", None), ["src/main.rs"]);
        assert_eq!(
            completions(dir.path(), "", None),
            ["Cargo.toml", "src/"],
            "no hidden ones unasked"
        );
        assert_eq!(completions(dir.path(), ".e", None), [".env"]);
        let home = dir.path().to_string_lossy().into_owned();
        assert_eq!(
            completions(std::path::Path::new("/"), "~/sr", Some(&home)),
            ["~/src/"]
        );
    }
}
