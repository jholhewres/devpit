use super::*;
use crate::ptyd_link::{is_ptyd, new_pane, pane_number, TMUX};

fn in_tmux(leaf_id: &str) -> Result<String, RpcError> {
    Ok(format!("devpit_x:{leaf_id}"))
}

/// A plain tab (`session.ensure`), an agent's or a card's (`ensure_at`) and
/// a split ask the same question, and nothing in it is what they are for.
#[cfg(unix)]
#[test]
fn a_plain_tab_an_agent_tab_and_a_split_go_to_ptyd_while_wanted_and_to_tmux_otherwise() {
    let daemon = crate::ptyd_link::tests::serving();
    let home = daemon.home.path();
    let store = Store::open(&home.join("state.db")).expect("store");
    let project = store.add_project(home, None).expect("project");
    let tabs = ["tab_plain".to_owned(), tab_for_card("card_1")];
    store
        .set_preference(devpit_core::preference::TERMINAL_BACKEND, TMUX)
        .expect("set");

    for tab in &tabs {
        let layout = first_layout(&store, Some(home), &project, tab, home, in_tmux).expect("tab");
        let (_, target) = layout.tree.leaves()[0];
        assert!(!is_ptyd(target), "{tab}: tmux makes it");
    }
    let split = new_pane(&store, Some(home), &project, "leaf_s", home, || {
        in_tmux("leaf_s")
    });
    assert!(!is_ptyd(&split.expect("split")), "a split: tmux");

    store
        .set_preference(devpit_core::preference::TERMINAL_BACKEND, "ptyd")
        .expect("set");
    let refused = |_: &str| -> Result<String, RpcError> {
        panic!("tmux was asked while the daemon answered")
    };
    let mut made = Vec::new();
    for tab in &tabs {
        let layout = first_layout(&store, Some(home), &project, tab, home, refused).expect("tab");
        let target = crate::pane_io::target_in(&layout, &layout.focused_id);
        assert!(is_ptyd(&target), "{tab}: {target}");
        made.push(layout.focused_id);
    }
    let split = new_pane(&store, Some(home), &project, "leaf_t", home, || {
        refused("leaf_t")
    });
    assert!(is_ptyd(&split.expect("split")));
    made.push("leaf_t".to_owned());

    for leaf in made {
        crate::ptyd_link::kill(home, pane_number(&leaf));
    }
}

/// The agent's line, typed into the pane a ptyd tab opened with — the
/// pane that was a tmux target nobody had made before.
#[cfg(unix)]
#[test]
fn a_line_typed_into_a_new_ptyd_tab_runs_in_its_shell() {
    use devpit_ptyd::frame::{self, Frame, Kind};

    let daemon = crate::ptyd_link::tests::serving();
    let home = daemon.home.path();
    let store = Store::open(&home.join("state.db")).expect("store");
    let project = store.add_project(home, None).expect("project");
    let layout = first_layout(&store, Some(home), &project, "tab_a", home, in_tmux).expect("tab");
    let pane = pane_number(&layout.focused_id);

    let mut watching = crate::ptyd_link::connect(home).expect("connect");
    frame::write(&mut watching, &Frame::new(pane, Kind::Attach, Vec::new())).expect("attach");
    watching
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .expect("timeout");
    let mut seen = Vec::new();
    let mut typed = false;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while std::time::Instant::now() < deadline && !seen.windows(8).any(|w| w == b"agent-42") {
        match frame::read(&mut watching) {
            Ok(Some(one)) if matches!(one.kind(), Some(Kind::Output | Kind::Replay)) => {
                seen.extend(one.bytes);
                // Once the shell has said something: typed sooner, a shell
                // still starting may drop it.
                if !typed {
                    typed = true;
                    crate::pane_io::write_at(
                        home,
                        pane,
                        &crate::pane_io::typed("echo agent-$((40+2))"),
                    )
                    .expect("typed");
                }
            }
            Ok(Some(one)) if one.kind() == Some(Kind::Refused) => {
                crate::ptyd_link::kill(home, pane);
                panic!("refused: {}", String::from_utf8_lossy(&one.bytes));
            }
            Ok(Some(_)) => {}
            _ => break,
        }
    }
    crate::ptyd_link::kill(home, pane);
    assert!(
        seen.windows(8).any(|w| w == b"agent-42"),
        "{}",
        String::from_utf8_lossy(&seen)
    );
}
