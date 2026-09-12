//! What a pane reports about itself, on the way to the window.
//!
//! One event and not five. `cwd`, `title`, `finished` and the rest are all
//! "something happened in this pane", and five channels would be five
//! subscriptions to the same question — with the screen having to keep them in
//! order itself.

/// What the window is told, in the shape it draws.
///
/// One event and not five, because they are all "something happened in this
/// pane" and five channels would be five subscriptions to the same question.
pub fn said(pane_id: &str, told: devpit_pty::Told) -> Happening {
    use devpit_pty::Told;
    let (what, detail) = match told {
        Told::Cwd(path) => ("cwd", Some(path)),
        Told::Title(title) => ("title", Some(title)),
        Told::PromptBegan => ("prompt", None),
        Told::OutputBegan => ("running", None),
        // Rendered as text rather than a number so the one event keeps one
        // shape. A code the screen wants to compare against zero parses it;
        // a code that was never reported is absent, and absent is not zero.
        Told::CommandEnded { code } => ("finished", code.map(|code| code.to_string())),
        Told::Clipboard(payload) => ("clipboard", Some(payload)),
    };
    Happening {
        pane_id: pane_id.to_owned(),
        what: what.to_owned(),
        detail,
    }
}

/// What an agent said about itself, in the pane it is running in.
///
/// The same event as everything else a pane reports, because it is the same
/// question — "what is happening in this terminal" — and the screen should
/// not have to subscribe twice and keep two answers in order itself.
///
/// The detail is the state, in the four words the agent CLIs actually report:
/// `working`, `waiting`, `done`. `waiting` is the one that matters most,
/// because nothing moves until somebody comes back to it.
pub fn agent_said(pane_id: &str, state: &str) -> Happening {
    Happening {
        pane_id: pane_id.to_owned(),
        what: "agent".to_owned(),
        detail: Some(state.to_owned()),
    }
}

/// The payload of `terminal:happening`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Happening {
    pub pane_id: String,
    /// `cwd` | `title` | `prompt` | `running` | `finished` | `clipboard` |
    /// `agent`.
    pub what: String,
    pub detail: Option<String>,
}

/// `terminal.happenings` — the shape `terminal:happening` carries.
///
/// It exists so the generated contract knows [`Happening`]: an event payload
/// is reachable from no command, and specta only writes down what a command
/// can reach. The same reason `chat.frames` exists.
#[tauri::command]
#[specta::specta]
pub fn terminal_happenings() -> Result<Vec<Happening>, devpit_rpc::RpcError> {
    Ok(Vec::new())
}
