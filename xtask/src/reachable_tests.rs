//! A pane nobody can open, caught before somebody finds out.

use super::*;

const LIST: &str = r#"
export const PANES: readonly PaneMeta[] = [
  { name: 'chat', title: 'Chat', label: 'Chat', icon: (<svg />) },
  { name: 'browser', title: 'Browser', label: 'Browser', icon: (<svg />) },
  { name: 'file', title: 'File', label: 'File', icon: (<svg />) },
]
"#;

const SIDEBAR: &str = r#"
  <button onClick={() => open('chat')}>Chat</button>
  <button onClick={() => open('browser')}>Browser</button>
"#;

#[test]
fn every_declared_pane_is_found() {
    assert_eq!(declared(LIST), ["chat", "browser", "file"]);
}

#[test]
fn every_pane_a_screen_opens_is_found_under_either_name() {
    assert_eq!(offered(SIDEBAR), ["chat", "browser"]);
    /* `show('x')` counts too: the Manager opens from the top bar and
    Capabilities from its own row, and both are perfectly reachable. */
    assert_eq!(offered("onClick={() => show('manager')}"), ["manager"]);
}

/// The one this guard exists for. The browser pane shipped through a whole
/// plan — mounted, tested, in the generated contract — reachable only from the
/// command palette, and what found it was the e2e saying `no button says
/// Browser`.
#[test]
fn a_pane_the_sidebar_forgot_is_named() {
    let sidebar = SIDEBAR.replace("open('browser')", "open('board')");
    assert_eq!(unreachable(LIST, &offered(&sidebar)), ["browser"]);
}

/// And the exemptions hold, so the guard does not shout about panes that are
/// opened by acting on something rather than by asking for one.
#[test]
fn a_pane_something_else_opens_is_left_alone() {
    /* `file` is in the list and in no sidebar, and that is correct. */
    assert!(unreachable(LIST, &offered(SIDEBAR)).is_empty());
}

/// The tree as it stands, which is the assertion that would have failed
/// before the browser pane was added to the sidebar.
#[test]
fn the_real_repository_can_open_every_pane_it_declares() {
    let found = every_pane_can_be_opened(&crate::workspace_root());
    assert!(
        found.is_empty(),
        "{}",
        found
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    );
}
