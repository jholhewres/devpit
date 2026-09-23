//! The pane tree of a project: what is open, where, and which one has focus.
//!
//! tmux owns the processes. This layer owns the shape they are arranged in,
//! and [`crate::panes`] owns a single live one. The split follows the two
//! questions: a drag changes the tree, and typing changes a pane.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use devpit_core::Store;
use devpit_git::worktree_path;
use devpit_rpc::{CardTerminal, ErrorCode, LayoutNode, RpcError, SessionLayout};
use tauri::State;
use ulid::Ulid;

use crate::claims::Claims;
use crate::shell_launch::wrapped_shell;
use crate::tap::listen;

pub struct SessionState {
    /// The handle a pane relays what it hears through.
    app: tauri::AppHandle,
    /// Who owns each pane right now. See [`crate::claims`].
    pub(crate) claims: Claims,
    /// The panes being listened to whether or not anyone is looking.
    /// See [`crate::tap`].
    pub(crate) taps: crate::tap::Taps,
    /// One lock per project, so two windows arranging two different projects
    /// do not queue behind each other.
    project_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl SessionState {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            claims: Claims::new(),
            taps: crate::tap::Taps::new(),
            project_locks: Mutex::new(HashMap::new()),
        }
    }

    /// Where a pane's news goes.
    pub(crate) fn app(&self) -> &tauri::AppHandle {
        &self.app
    }

    pub(crate) fn project_lock(&self, project_id: &str) -> Result<Arc<Mutex<()>>, RpcError> {
        let mut locks = self
            .project_locks
            .lock()
            .map_err(|_| RpcError::internal("project lock registry"))?;
        Ok(Arc::clone(
            locks
                .entry(project_id.to_owned())
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        ))
    }
}

pub(crate) fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

pub(crate) fn tmux_server() -> Result<devpit_tmux::Server, RpcError> {
    if !devpit_tmux::Server::available() {
        return Err(RpcError::new(
            ErrorCode::Unsupported,
            "tmux is not installed, or not on PATH",
        ));
    }
    Ok(devpit_tmux::Server::new(socket_path()?).with_shell(wrapped_shell()?))
}

/// The tmux socket, at the state root and never under a per-project directory.
///
/// A Unix socket path is capped at ~108 bytes by `sun_path`, and going over it
/// fails at connect time with "File name too long" — a symptom that points
/// nowhere near the decision that caused it. Which is why it hangs off the
/// state root and not off a project's directory, whose name is somebody's
/// folder and as long as they like.
fn socket_path() -> Result<PathBuf, RpcError> {
    Ok(Store::root()?.join("tmux.sock"))
}

pub(crate) fn locate_cwd(project_id: &str, worktree_id: Option<&str>) -> Result<PathBuf, RpcError> {
    let store = store()?;
    let row = store
        .project(project_id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that project is not registered"))?;
    let root = PathBuf::from(&row.root_path);
    if !root.is_dir() {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("{} is no longer on disk", row.root_path),
        ));
    }
    if let Some(id) = worktree_id {
        if let Some(path) =
            worktree_path(&root, id).map_err(|err| RpcError::internal(err.to_string()))?
        {
            return Ok(path);
        }
    }
    Ok(root)
}

pub(crate) fn encode(layout: &SessionLayout) -> Result<String, RpcError> {
    serde_json::to_string(&layout.tree).map_err(|err| RpcError::internal(err.to_string()))
}

/// Reads a stored tree, naming any boundary that predates boundary ids.
///
/// The naming happens on every read and costs one walk. Persisting it is the
/// caller's business: `decode` is used from paths that only look, and a read
/// that writes would turn opening a window into a disk write.
pub(crate) fn decode(
    project_id: &str,
    tree: &str,
    focused_id: &str,
) -> Result<SessionLayout, RpcError> {
    let mut tree: LayoutNode =
        serde_json::from_str(tree).map_err(|err| RpcError::internal(err.to_string()))?;
    tree.name_the_splits(&mut || format!("sp_{}", Ulid::generate()));
    Ok(SessionLayout {
        project_id: project_id.to_owned(),
        focused_id: focused_id.to_owned(),
        tree,
    })
}

/// The layout a project has, or the error that says it has none yet.
///
/// One place, because four commands asked the same question in four ways and
/// two of them phrased the missing case differently.
pub(crate) fn layout_of(project_id: &str, tab_id: &str) -> Result<SessionLayout, RpcError> {
    let Some((tree, focused)) = store()?.pane_layout(project_id, tab_id)? else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "this project has no session yet",
        ));
    };
    decode(project_id, &tree, &focused)
}

/// The argv a pty spawns to become a client of this leaf's tmux window.
pub(crate) fn attach_argv(project_id: &str, leaf_id: &str) -> Result<Vec<String>, RpcError> {
    Ok(tmux_server()?.attach_argv(&devpit_tmux::Server::session_name(project_id), leaf_id))
}

pub(crate) fn persist(store: &Store, tab_id: &str, layout: &SessionLayout) -> Result<(), RpcError> {
    store.set_pane_layout(
        &layout.project_id,
        tab_id,
        &encode(layout)?,
        &layout.focused_id,
    )?;
    Ok(())
}

fn load_or_create(project_id: &str, tab_id: &str, cwd: &Path) -> Result<SessionLayout, RpcError> {
    let store = store()?;
    let server = tmux_server()?;
    let session = devpit_tmux::Server::session_name(project_id);

    if let Some((tree, focused)) = store.pane_layout(project_id, tab_id)? {
        let layout = decode(project_id, &tree, &focused)?;
        // A window missing before this is one tmux lost with its server, not
        // one closed on purpose — closing forgets the pane's layout too.
        let had = match server.has_session(&session).map_err(tmux_err)? {
            true => server.list_windows(&session).map_err(tmux_err)?,
            false => Vec::new(),
        };
        for (leaf_id, _) in layout.tree.leaves() {
            server
                .ensure_session(&session, leaf_id, cwd)
                .map_err(tmux_err)?;
            if !had.iter().any(|name| name == leaf_id) {
                crate::restoring::start_again(&store, &session, leaf_id);
            }
        }
        return Ok(layout);
    }

    let leaf_id = format!("leaf_{}", Ulid::generate());
    server
        .ensure_session(&session, &leaf_id, cwd)
        .map_err(tmux_err)?;
    let layout = SessionLayout {
        project_id: project_id.to_owned(),
        focused_id: leaf_id.clone(),
        tree: LayoutNode::leaf(
            leaf_id.clone(),
            devpit_tmux::Server::target(&session, &leaf_id),
        ),
    };
    persist(&store, tab_id, &layout)?;
    Ok(layout)
}

pub(crate) fn tmux_err(err: devpit_tmux::TmuxError) -> RpcError {
    match err {
        devpit_tmux::TmuxError::Missing => RpcError::new(
            ErrorCode::Unsupported,
            "tmux is not installed, or not on PATH",
        ),
        other => RpcError::internal(other.to_string()),
    }
}

/// `session.ensure` — a layout and a tmux window, created if they were missing.
///
/// Async so it does not run on the thread that draws the window.
///
/// It opens the store and spawns tmux several times — a handful of
/// milliseconds each, and all of them on the main thread while somebody
/// watches an empty pane. There is no `await` in the body, so the work still
/// happens in one go; it just happens somewhere the window can paint through.
#[tauri::command]
#[specta::specta]
pub async fn session_ensure(
    state: State<'_, SessionState>,
    project_id: String,
    tab_id: String,
    worktree_id: Option<String>,
) -> Result<SessionLayout, RpcError> {
    let cwd = locate_cwd(&project_id, worktree_id.as_deref())?;
    ensure_at(&state, &project_id, &tab_id, &cwd)
}

/// The tab a card's terminal lives in.
///
/// Derived from the card rather than minted, so opening it twice lands in the
/// same place — and stated once, because two `format!`s that have to agree
/// are two `format!`s that one day will not.
pub(crate) fn tab_for_card(card_id: &str) -> String {
    format!("{CARD_TAB}{card_id}")
}

const CARD_TAB: &str = "tab_card_";

/// Whether a tab id is in the card namespace, plain id or not.
pub(crate) fn is_card_tab(tab_id: &str) -> bool {
    tab_id.starts_with(CARD_TAB)
}

/// The card a tab was named for, when it was — and only a plain id: a tab id
/// comes from the window, which can send anything.
pub(crate) fn card_of_tab(tab_id: &str) -> Option<&str> {
    tab_id
        .strip_prefix(CARD_TAB)
        .filter(|card| crate::adopting::plain(card))
}

/// Opens (or reopens) a tab whose panes start in `cwd`.
///
/// The body `session_ensure` had, lifted so a card can ask for a terminal in
/// its own checkout — which is a directory, not a worktree id, and so could
/// not go through `locate_cwd`.
pub(crate) fn ensure_at(
    state: &State<'_, SessionState>,
    project_id: &str,
    tab_id: &str,
    cwd: &Path,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;
    let layout = load_or_create(project_id, tab_id, cwd)?;
    listen(state, project_id, &layout);
    Ok(layout)
}

/// `session.focus` — persists which leaf receives the next split or action.
#[tauri::command]
#[specta::specta]
pub async fn session_focus(
    app: tauri::AppHandle,
    project_id: String,
    tab_id: String,
    leaf_id: String,
) -> Result<SessionLayout, RpcError> {
    crate::off_main::blocking(move || {
        let state = tauri::Manager::state::<SessionState>(&app);
        session_focus_now(state, project_id, tab_id, leaf_id)
    })
    .await
}

/// [`session_focus`], on the calling thread.
pub(crate) fn session_focus_now(
    state: State<SessionState>,
    project_id: String,
    tab_id: String,
    leaf_id: String,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;
    let mut layout = layout_of(&project_id, &tab_id)?;
    layout.focused_id = leaf_id.clone();
    if !layout.tree.contains_leaf(&leaf_id) {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that pane is not in this project's layout",
        ));
    }
    persist(&store()?, &tab_id, &layout)?;
    Ok(layout)
}

/// `terminal.attach_agent` — brings a card's background session into the
/// card's own terminal, in its checkout.
///
/// The session that was there keeps running detached; attaching takes up the
/// screen, not the work. The line is typed into the tab's focused pane, so a
/// pane running something else is refused rather than typed over — and this is
/// only ever an action of the interface, never a side effect of a drag.
#[tauri::command]
#[specta::specta]
pub async fn terminal_attach_agent(
    state: State<'_, SessionState>,
    project_id: String,
    card_id: String,
) -> Result<CardTerminal, RpcError> {
    let (project, wanted) = (project_id.clone(), card_id.clone());
    let (short_id, checkout, runner) = tauri::async_runtime::spawn_blocking(move || {
        let store = store()?;
        if store.live_card_project(&wanted)?.as_deref() != Some(project.as_str()) {
            return Err(RpcError::new(
                ErrorCode::NotFound,
                "no such card in this project",
            ));
        }
        let link = store
            .session_link(&wanted)?
            .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "this card has no session yet"))?;
        let checkout = crate::checkout::checkout_of(&store, &wanted, |_| {})
            .map_err(|why| RpcError::new(ErrorCode::Internal, why))?;
        // Started under a profile, attached under the same one: the binary and
        // the account are the session's, not this moment's default.
        let runner = link
            .profile_id
            .as_deref()
            .map(|id| crate::agent_profiles::runner_for(&store, id))
            .transpose()
            .map_err(|why| RpcError::new(ErrorCode::NotFound, why))?;
        Ok::<_, RpcError>((link.short_id, checkout, runner))
    })
    .await
    .map_err(|err| RpcError::internal(err.to_string()))??;

    // The card's own tab, made at its checkout if it is not there yet.
    let tab_id = tab_for_card(&card_id);
    let layout = ensure_at(&state, &project_id, &tab_id, &checkout)?;
    let session = devpit_tmux::Server::session_name(&project_id);
    let target = devpit_tmux::Server::target(&session, &layout.focused_id);

    // Not under the project lock: a shell reaching its prompt takes seconds.
    let (waiting, leaf) = (session.clone(), layout.focused_id.clone());
    let ready =
        tauri::async_runtime::spawn_blocking(move || crate::shell_launch::settled(&waiting, &leaf))
            .await
            .map_err(|err| RpcError::internal(err.to_string()))?;
    let line = crate::attaching::attach_target(&checkout, &short_id, runner.as_ref(), &ready)?;

    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;
    tmux_server()?.send_keys(&target, &line).map_err(tmux_err)?;
    Ok(CardTerminal {
        layout,
        tab_id,
        card_id,
    })
}

/// The layout of whichever tab holds this leaf.
///
/// Attaching to a pane needs to know the pane is this project's, not which tab
/// it is drawn in — and a caller that had to name the tab would be carrying an
/// answer it does not have when a pane is reached from a card or a shortcut.
pub(crate) fn holding(project_id: &str, leaf_id: &str) -> Result<SessionLayout, RpcError> {
    // One connection for the whole walk. `layout_of` opens its own, so asking
    // it per tab opened the database once per tab a project has — on the path
    // that attaches a terminal, which is the one nobody is willing to wait on.
    let store = store()?;
    for tab_id in store.pane_layout_tabs(project_id)? {
        let Ok(Some((tree, focused))) = store.pane_layout(project_id, &tab_id) else {
            continue;
        };
        if let Ok(layout) = decode(project_id, &tree, &focused) {
            if layout.tree.contains_leaf(leaf_id) {
                return Ok(layout);
            }
        }
    }
    Err(RpcError::new(
        ErrorCode::NotFound,
        "that pane is not in this project",
    ))
}
