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
use devpit_rpc::{ErrorCode, LayoutNode, RpcError, SessionLayout, SplitDirection};
use tauri::State;
use ulid::Ulid;

use crate::claims::Claims;

pub struct SessionState {
    /// The handle a pane relays what it hears through.
    app: tauri::AppHandle,
    /// Who owns each pane right now. See [`crate::claims`].
    pub(crate) claims: Claims,
    /// One lock per project, so two windows arranging two different projects
    /// do not queue behind each other.
    project_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl SessionState {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            claims: Claims::new(),
            project_locks: Mutex::new(HashMap::new()),
        }
    }

    /// Where a pane's news goes.
    pub(crate) fn app(&self) -> &tauri::AppHandle {
        &self.app
    }

    fn project_lock(&self, project_id: &str) -> Result<Arc<Mutex<()>>, RpcError> {
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

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

fn tmux_server() -> Result<devpit_tmux::Server, RpcError> {
    if !devpit_tmux::Server::available() {
        return Err(RpcError::new(
            ErrorCode::Unsupported,
            "tmux is not installed, or not on PATH",
        ));
    }
    Ok(devpit_tmux::Server::new(socket_path()?))
}

/// The tmux socket, at the state root and never under a per-project directory.
///
/// A Unix socket path is capped at ~108 bytes by `sun_path`, and going over it
/// fails at connect time with "File name too long" — a symptom that points
/// nowhere near the decision that caused it. `the_tmux_socket_path_stays_short`
/// fails if this ever grows.
fn socket_path() -> Result<PathBuf, RpcError> {
    Ok(Store::root()?.join("tmux.sock"))
}

fn locate_cwd(project_id: &str, worktree_id: Option<&str>) -> Result<PathBuf, RpcError> {
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

fn encode(layout: &SessionLayout) -> Result<String, RpcError> {
    serde_json::to_string(&layout.tree).map_err(|err| RpcError::internal(err.to_string()))
}

/// Reads a stored tree, naming any boundary that predates boundary ids.
///
/// The naming happens on every read and costs one walk. Persisting it is the
/// caller's business: `decode` is used from paths that only look, and a read
/// that writes would turn opening a window into a disk write.
fn decode(project_id: &str, tree: &str, focused_id: &str) -> Result<SessionLayout, RpcError> {
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
pub(crate) fn layout_of(project_id: &str) -> Result<SessionLayout, RpcError> {
    let Some((tree, focused)) = store()?.pane_layout(project_id)? else {
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

fn persist(store: &Store, layout: &SessionLayout) -> Result<(), RpcError> {
    store.set_pane_layout(&layout.project_id, &encode(layout)?, &layout.focused_id)?;
    Ok(())
}

fn load_or_create(project_id: &str, cwd: &Path) -> Result<SessionLayout, RpcError> {
    let store = store()?;
    let server = tmux_server()?;
    let session = devpit_tmux::Server::session_name(project_id);

    if let Some((tree, focused)) = store.pane_layout(project_id)? {
        let layout = decode(project_id, &tree, &focused)?;
        for (leaf_id, _) in layout.tree.leaves() {
            server
                .ensure_session(&session, leaf_id, cwd)
                .map_err(tmux_err)?;
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
    persist(&store, &layout)?;
    Ok(layout)
}

fn tmux_err(err: devpit_tmux::TmuxError) -> RpcError {
    match err {
        devpit_tmux::TmuxError::Missing => RpcError::new(
            ErrorCode::Unsupported,
            "tmux is not installed, or not on PATH",
        ),
        other => RpcError::internal(other.to_string()),
    }
}

/// `session.ensure` — a layout and a tmux window, created if they were missing.
#[tauri::command]
#[specta::specta]
pub fn session_ensure(
    state: State<SessionState>,
    project_id: String,
    worktree_id: Option<String>,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;
    let cwd = locate_cwd(&project_id, worktree_id.as_deref())?;
    load_or_create(&project_id, &cwd)
}

/// `session.layout` — the tree as last persisted.
#[tauri::command]
#[specta::specta]
pub fn session_layout(project_id: String) -> Result<SessionLayout, RpcError> {
    layout_of(&project_id)
}

/// `session.focus` — persists which leaf receives the next split or action.
#[tauri::command]
#[specta::specta]
pub fn session_focus(
    state: State<SessionState>,
    project_id: String,
    leaf_id: String,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;
    let mut layout = layout_of(&project_id)?;
    layout.focused_id = leaf_id.clone();
    if !layout.tree.contains_leaf(&leaf_id) {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that pane is not in this project's layout",
        ));
    }
    persist(&store()?, &layout)?;
    Ok(layout)
}

/// `session.split` — a new tmux window and a split node in the tree.
#[tauri::command]
#[specta::specta]
pub fn session_split(
    state: State<SessionState>,
    project_id: String,
    leaf_id: String,
    direction: SplitDirection,
    worktree_id: Option<String>,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;
    let cwd = locate_cwd(&project_id, worktree_id.as_deref())?;
    let server = tmux_server()?;
    let session = devpit_tmux::Server::session_name(&project_id);

    let current = layout_of(&project_id)?;
    if !current.tree.contains_leaf(&leaf_id) {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that pane is not in this project's layout",
        ));
    }

    let new_id = format!("leaf_{}", Ulid::generate());
    server
        .new_window(&session, &new_id, &cwd)
        .map_err(tmux_err)?;
    let new_leaf = LayoutNode::leaf(
        new_id.clone(),
        devpit_tmux::Server::target(&session, &new_id),
    );
    let tree = current
        .tree
        .split_leaf(&leaf_id, direction, new_leaf)
        .ok_or_else(|| RpcError::internal("the leaf vanished while splitting"))?;

    let layout = SessionLayout {
        project_id,
        focused_id: new_id,
        tree,
    };
    persist(&store()?, &layout)?;
    Ok(layout)
}

/// `session.close_leaf` — the pane goes, and its tmux window with it.
///
/// The hole this fills: `session.split` could only ever add. A tree that only
/// grows is a leak wearing a layout's clothes.
///
/// Refuses the last pane. A session with no pane is not a layout, and the
/// refusal says so rather than persisting an empty tree the screen cannot draw.
#[tauri::command]
#[specta::specta]
pub fn session_close_leaf(
    state: State<SessionState>,
    project_id: String,
    leaf_id: String,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;

    let current = layout_of(&project_id)?;
    if !current.tree.contains_leaf(&leaf_id) {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that pane is not in this project's layout",
        ));
    }
    let tree = current.tree.close_leaf(&leaf_id).ok_or_else(|| {
        RpcError::new(
            ErrorCode::Conflict,
            "that is the only pane left — close the project instead",
        )
    })?;

    // The window goes after the tree is known to be closable, so a refusal
    // never leaves a session whose layout and tmux disagree.
    let session = devpit_tmux::Server::session_name(&project_id);
    tmux_server()?
        .kill_window(&session, &leaf_id)
        .map_err(tmux_err)?;

    // Focus follows the tree when it pointed at what just left.
    let focused_id = if current.focused_id == leaf_id {
        tree.first_leaf_id().to_owned()
    } else {
        current.focused_id
    };
    let layout = SessionLayout {
        project_id,
        focused_id,
        tree,
    };
    persist(&store()?, &layout)?;
    Ok(layout)
}

/// `session.rename_leaf` — the name the person gave this pane.
///
/// An empty name clears it, which is how a pane goes back to showing what the
/// program running in it calls itself. The person's name always wins over the
/// program's: a title escape arriving later must not undo a rename.
#[tauri::command]
#[specta::specta]
pub fn session_rename_leaf(
    state: State<SessionState>,
    project_id: String,
    leaf_id: String,
    name: String,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;

    let current = layout_of(&project_id)?;
    let tree = current.tree.rename_leaf(&leaf_id, &name).ok_or_else(|| {
        RpcError::new(
            ErrorCode::NotFound,
            "that pane is not in this project's layout",
        )
    })?;
    let layout = SessionLayout {
        project_id,
        focused_id: current.focused_id,
        tree,
    };
    persist(&store()?, &layout)?;
    Ok(layout)
}

/// `session.set_ratio` — where a boundary was dragged to.
///
/// Persisted because Orca's rule is the right one: boundaries stay where you
/// put them, and resizing the window does not shuffle a layout someone
/// arranged. The tree clamps, so neither side can be dragged out of reach.
#[tauri::command]
#[specta::specta]
pub fn session_set_ratio(
    state: State<SessionState>,
    project_id: String,
    split_id: String,
    ratio: f64,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;

    let current = layout_of(&project_id)?;
    let tree = current.tree.set_ratio(&split_id, ratio).ok_or_else(|| {
        RpcError::new(
            ErrorCode::NotFound,
            "no such boundary in this project's layout",
        )
    })?;
    let layout = SessionLayout {
        project_id,
        focused_id: current.focused_id,
        tree,
    };
    persist(&store()?, &layout)?;
    Ok(layout)
}

/// `terminal.attach_agent` — brings a card's session into the target terminal.
///
/// This is the rule the product turns on: one target terminal per project, and
/// switching cards switches what is attached to it. The session that was there
/// keeps running detached; it stops taking up the screen, not working.
///
/// The command is typed into the focused pane, which means typing over
/// whoever is sitting there — so this is only ever an action of the interface,
/// with the text in front of the person, never a side effect of a drag.
#[tauri::command]
#[specta::specta]
pub fn terminal_attach_agent(
    state: State<SessionState>,
    project_id: String,
    card_id: String,
) -> Result<String, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;

    let store = store()?;
    let link = store
        .session_link(&card_id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "this card has no session yet"))?;

    // The terminal is made if it is not there yet. Refusing because nobody has
    // opened one is refusing over a step the product can take itself — and the
    // person clicking this on a card is asking for exactly that terminal.
    let layout = match layout_of(&project_id) {
        Ok(layout) => layout,
        Err(_) => {
            let cwd = locate_cwd(&project_id, None)?;
            load_or_create(&project_id, &cwd)?
        }
    };

    let session = devpit_tmux::Server::session_name(&project_id);
    let target = devpit_tmux::Server::target(&session, &layout.focused_id);

    let line = devpit_agentcli::attach_argv(&link.short_id).join(" ");
    tmux_server()?.send_keys(&target, &line).map_err(tmux_err)?;

    Ok(line)
}
