//! Session commands: tmux owns the process, this layer attaches a client.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize};
use quockpit_core::Store;
use quockpit_git::worktree_path;
use quockpit_rpc::{ErrorCode, LayoutNode, RpcError, SessionLayout, SplitDirection};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::State;
use ulid::Ulid;

pub struct SessionState {
    lives: Mutex<HashMap<String, Arc<Live>>>,
    claims: Mutex<HashMap<String, String>>,
    latest_clients: Mutex<HashMap<String, String>>,
    project_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

struct Live {
    client_id: String,
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            lives: Mutex::new(HashMap::new()),
            claims: Mutex::new(HashMap::new()),
            latest_clients: Mutex::new(HashMap::new()),
            project_locks: Mutex::new(HashMap::new()),
        }
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

    /// Claims the pane for a newer UI mount and returns its previous client.
    fn claim_client(&self, pane_id: &str, client_id: &str) -> Result<Option<Arc<Live>>, RpcError> {
        let mut latest = self
            .latest_clients
            .lock()
            .map_err(|_| RpcError::internal("client generation lock"))?;
        if latest
            .get(pane_id)
            .is_some_and(|current| current.as_str() >= client_id)
        {
            return Err(RpcError::new(
                ErrorCode::Conflict,
                "a newer client already owns that pane",
            ));
        }
        latest.insert(pane_id.to_owned(), client_id.to_owned());
        let mut claims = self
            .claims
            .lock()
            .map_err(|_| RpcError::internal("session claim lock"))?;
        claims.insert(pane_id.to_owned(), client_id.to_owned());
        drop(claims);
        drop(latest);

        Ok(self
            .lives
            .lock()
            .map_err(|_| RpcError::internal("session lock"))?
            .remove(pane_id))
    }

    fn install_client(
        &self,
        pane_id: &str,
        client_id: &str,
        live: Arc<Live>,
    ) -> Result<bool, RpcError> {
        let claims = self
            .claims
            .lock()
            .map_err(|_| RpcError::internal("session claim lock"))?;
        if claims
            .get(pane_id)
            .is_none_or(|current| current != client_id)
        {
            return Ok(false);
        }
        self.lives
            .lock()
            .map_err(|_| RpcError::internal("session lock"))?
            .insert(pane_id.to_owned(), live);
        Ok(true)
    }

    fn release_claim(&self, pane_id: &str, client_id: &str) -> Result<bool, RpcError> {
        let mut claims = self
            .claims
            .lock()
            .map_err(|_| RpcError::internal("session claim lock"))?;
        if claims
            .get(pane_id)
            .is_some_and(|current| current == client_id)
        {
            claims.remove(pane_id);
            return Ok(true);
        }
        Ok(false)
    }
}

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

fn tmux_server() -> Result<quockpit_tmux::Server, RpcError> {
    if !quockpit_tmux::Server::available() {
        return Err(RpcError::new(
            ErrorCode::Unsupported,
            "tmux is not installed, or not on PATH",
        ));
    }
    Ok(quockpit_tmux::Server::new(socket_path()?))
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

fn decode(project_id: &str, tree: &str, focused_id: &str) -> Result<SessionLayout, RpcError> {
    let tree: LayoutNode =
        serde_json::from_str(tree).map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(SessionLayout {
        project_id: project_id.to_owned(),
        focused_id: focused_id.to_owned(),
        tree,
    })
}

fn persist(store: &Store, layout: &SessionLayout) -> Result<(), RpcError> {
    store.set_pane_layout(&layout.project_id, &encode(layout)?, &layout.focused_id)?;
    Ok(())
}

fn load_or_create(project_id: &str, cwd: &Path) -> Result<SessionLayout, RpcError> {
    let store = store()?;
    let server = tmux_server()?;
    let session = quockpit_tmux::Server::session_name(project_id);

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
            quockpit_tmux::Server::target(&session, &leaf_id),
        ),
    };
    persist(&store, &layout)?;
    Ok(layout)
}

fn tmux_err(err: quockpit_tmux::TmuxError) -> RpcError {
    match err {
        quockpit_tmux::TmuxError::Missing => RpcError::new(
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
    let store = store()?;
    let Some((tree, focused)) = store.pane_layout(&project_id)? else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "this project has no session yet",
        ));
    };
    decode(&project_id, &tree, &focused)
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
    let store = store()?;
    let Some((tree, _)) = store.pane_layout(&project_id)? else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "this project has no session yet",
        ));
    };
    let layout = decode(&project_id, &tree, &leaf_id)?;
    if !layout.tree.contains_leaf(&leaf_id) {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that pane is not in this project's layout",
        ));
    }
    persist(&store, &layout)?;
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
    let store = store()?;
    let server = tmux_server()?;
    let session = quockpit_tmux::Server::session_name(&project_id);

    let Some((tree, focused)) = store.pane_layout(&project_id)? else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "this project has no session yet",
        ));
    };
    let current = decode(&project_id, &tree, &focused)?;
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
        quockpit_tmux::Server::target(&session, &new_id),
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
    persist(&store, &layout)?;
    Ok(layout)
}

/// `session.write` — bytes into the attached client of a leaf.
#[tauri::command]
#[specta::specta]
pub fn session_write(
    state: State<SessionState>,
    pane_id: String,
    data: String,
) -> Result<(), RpcError> {
    let live = {
        let lives = state
            .lives
            .lock()
            .map_err(|_| RpcError::internal("session lock"))?;
        lives
            .get(&pane_id)
            .cloned()
            .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that pane is not attached"))?
    };
    let mut writer = live
        .writer
        .lock()
        .map_err(|_| RpcError::internal("writer lock"))?;
    writer
        .write_all(data.as_bytes())
        .and_then(|_| writer.flush())
        .map_err(|err| RpcError::internal(err.to_string()))
}

/// `session.resize` — the pty size of an attached leaf.
#[tauri::command]
#[specta::specta]
pub fn session_resize(
    state: State<SessionState>,
    pane_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), RpcError> {
    let live = {
        let lives = state
            .lives
            .lock()
            .map_err(|_| RpcError::internal("session lock"))?;
        lives
            .get(&pane_id)
            .cloned()
            .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that pane is not attached"))?
    };
    let result = live
        .master
        .lock()
        .map_err(|_| RpcError::internal("master lock"))?
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| RpcError::internal(err.to_string()));
    result
}

/// `session.detach` — closes only this app's client; tmux keeps the shell.
#[tauri::command]
#[specta::specta]
pub fn session_detach(
    state: State<SessionState>,
    pane_id: String,
    client_id: String,
) -> Result<(), RpcError> {
    let owns_claim = state.release_claim(&pane_id, &client_id)?;
    if !owns_claim {
        return Ok(());
    }
    let live = {
        let mut lives = state
            .lives
            .lock()
            .map_err(|_| RpcError::internal("session lock"))?;
        match lives.get(&pane_id) {
            Some(live) if live.client_id == client_id => lives.remove(&pane_id),
            _ => None,
        }
    };
    if let Some(live) = live {
        live.killer
            .lock()
            .map_err(|_| RpcError::internal("killer lock"))?
            .kill()
            .map_err(|err| RpcError::internal(err.to_string()))?;
    }
    Ok(())
}

/// Attaches a client pty to the tmux window for this leaf and streams frames.
///
/// Not in the generated contract: the binary channel cannot be described by
/// specta. The wrapper lives next to the xterm host, like `pty_drain`.
#[tauri::command]
pub async fn session_attach(
    state: State<'_, SessionState>,
    project_id: String,
    pane_id: String,
    client_id: String,
    rows: u16,
    cols: u16,
    on_frame: Channel<InvokeResponseBody>,
) -> Result<(), RpcError> {
    let store = store()?;
    let Some((tree, focused)) = store.pane_layout(&project_id)? else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "this project has no session yet",
        ));
    };
    let layout = decode(&project_id, &tree, &focused)?;
    if !layout.tree.contains_leaf(&pane_id) {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that pane is not in this layout",
        ));
    }

    let server = tmux_server()?;
    let argv = server.attach_argv(&quockpit_tmux::Server::session_name(&project_id), &pane_id);

    let mut builder = CommandBuilder::new(&argv[0]);
    for arg in &argv[1..] {
        builder.arg(arg);
    }
    builder.env("TERM", "xterm-256color");

    if let Some(previous) = state.claim_client(&pane_id, &client_id)? {
        let _ = previous
            .killer
            .lock()
            .map_err(|_| RpcError::internal("killer lock"))?
            .kill();
    }

    let mut session = match quockpit_pty::spawn(
        builder,
        PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        },
    ) {
        Ok(session) => session,
        Err(err) => {
            state.release_claim(&pane_id, &client_id)?;
            return Err(RpcError::internal(err.to_string()));
        }
    };

    let Some(io) = session.take_io() else {
        state.release_claim(&pane_id, &client_id)?;
        return Err(RpcError::internal("the pty did not expose its io handles"));
    };
    let live = Arc::new(Live {
        client_id: client_id.clone(),
        writer: Mutex::new(io.writer),
        master: Mutex::new(io.master),
        killer: Mutex::new(io.killer),
    });
    if !state.install_client(&pane_id, &client_id, Arc::clone(&live))? {
        let _ = live
            .killer
            .lock()
            .map_err(|_| RpcError::internal("killer lock"))?
            .kill();
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "a newer client already owns that pane",
        ));
    }

    while let Some(frame) = session.frames.recv().await {
        if on_frame.send(InvokeResponseBody::Raw(frame)).is_err() {
            break;
        }
    }

    if let Ok(mut lives) = state.lives.lock() {
        let still_ours = lives
            .get(&pane_id)
            .is_some_and(|current| Arc::ptr_eq(current, &live));
        if still_ours {
            lives.remove(&pane_id);
        }
    }
    let _ = state.release_claim(&pane_id, &client_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_older_attach_cannot_replace_a_newer_claim() {
        let state = SessionState::new();
        assert!(state
            .claim_client("leaf", "0000000000000100")
            .expect("first claim")
            .is_none());

        let stale = state.claim_client("leaf", "0000000000000099");
        assert!(matches!(
            stale,
            Err(RpcError {
                code: ErrorCode::Conflict,
                ..
            })
        ));

        assert!(state
            .claim_client("leaf", "0000000000000101")
            .expect("newer claim")
            .is_none());
        assert!(state
            .release_claim("leaf", "0000000000000101")
            .expect("release"));
        assert!(
            state.claim_client("leaf", "0000000000000100").is_err(),
            "an old invoke arrived after detach and reclaimed the pane"
        );
    }

    /// The guard that keeps the socket reachable.
    ///
    /// Moving it under a per-project directory is the tempting change that
    /// breaks it, and `connect(2)` reports that as "File name too long"
    /// without naming the cause.
    #[test]
    fn the_tmux_socket_path_stays_short() {
        let path = socket_path().expect("socket path");
        let bytes = path.as_os_str().len();
        assert!(
            bytes <= 100,
            "the tmux socket path is {bytes} bytes, past the ~108 the kernel allows: {}",
            path.display()
        );
    }

    #[test]
    fn projects_have_independent_mutation_locks() {
        let state = SessionState::new();
        let a = state.project_lock("a").expect("a");
        let a_again = state.project_lock("a").expect("a again");
        let b = state.project_lock("b").expect("b");

        assert!(Arc::ptr_eq(&a, &a_again));
        assert!(!Arc::ptr_eq(&a, &b));
    }
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
    let layout = match store.pane_layout(&project_id)? {
        Some((tree, focused)) => decode(&project_id, &tree, &focused)?,
        None => {
            let cwd = locate_cwd(&project_id, None)?;
            load_or_create(&project_id, &cwd)?
        }
    };

    let session = quockpit_tmux::Server::session_name(&project_id);
    let target = quockpit_tmux::Server::target(&session, &layout.focused_id);

    let line = quockpit_agentcli::attach_argv(&link.short_id).join(" ");
    tmux_server()?.send_keys(&target, &line).map_err(tmux_err)?;

    Ok(line)
}
