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

/// The payload of `terminal:happening`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Happening {
    pub pane_id: String,
    /// `cwd` | `title` | `prompt` | `running` | `finished` | `clipboard`.
    pub what: String,
    pub detail: Option<String>,
}
