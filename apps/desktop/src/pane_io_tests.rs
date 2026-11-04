use super::*;
use devpit_rpc::{LayoutNode, SplitDirection};

#[test]
fn a_target_says_whether_tmux_or_the_daemon_holds_the_pane() {
    assert_eq!(
        holder("devpit_p1:leaf_a"),
        Holder::Tmux {
            session: "devpit_p1",
            window: "leaf_a"
        }
    );
    assert_eq!(
        holder(&crate::ptyd_link::target("leaf_a")),
        Holder::Ptyd(crate::ptyd_link::pane_number("leaf_a"))
    );
}

#[test]
fn a_tmux_leaf_is_reached_by_its_names_and_a_ptyd_leaf_by_its_target() {
    let tree = LayoutNode::leaf("leaf_a", "an-older-spelling")
        .split_leaf(
            "leaf_a",
            SplitDirection::Vertical,
            LayoutNode::leaf("leaf_b", crate::ptyd_link::target("leaf_b")),
        )
        .expect("split");
    let layout = SessionLayout {
        project_id: "p1".into(),
        focused_id: "leaf_a".into(),
        tree,
    };
    let session = devpit_tmux::Server::session_name("p1");
    assert_eq!(
        target_in(&layout, "leaf_a"),
        devpit_tmux::Server::target(&session, "leaf_a")
    );
    assert_eq!(target_in(&layout, "leaf_b"), "ptyd:leaf_b");
}

#[test]
fn a_submitted_line_clears_the_line_and_screen_then_runs() {
    assert_eq!(submitted("ls -la"), b"\x05\x15\x0cls -la\r");
    assert_eq!(
        submitted("for x in 1 2\ndo echo $x; done"),
        b"\x05\x15\x0c\x1b[200~for x in 1 2\rdo echo $x; done\x1b[201~\r"
    );
}

#[test]
fn only_several_lines_go_as_a_bracketed_paste() {
    assert_eq!(pasted("one line", false), b"one line");
    assert_eq!(pasted("one line", true), b"one line\r");
    assert_eq!(pasted("a\r\nb", true), b"\x1b[200~a\rb\x1b[201~\r");
}

#[test]
fn a_paste_cannot_end_its_own_bracket() {
    assert_eq!(
        pasted("a\x1b[201~\nrm -rf ~", false),
        b"\x1b[200~a\rrm -rf ~\x1b[201~"
    );
}

#[test]
fn keys_are_the_bytes_a_terminal_sends_for_them() {
    assert_eq!(
        pressed(&[Key::Up, Key::Down, Key::Enter, Key::Escape]),
        b"\x1b[A\x1b[B\r\x1b"
    );
}

/// Over the real socket: what is written reaches the pane's program, with
/// no answer waited for.
#[cfg(unix)]
#[test]
fn a_line_written_to_a_ptyd_pane_runs_in_it() {
    use devpit_ptyd::frame::{self, Frame, Kind};

    let daemon = crate::ptyd_link::tests::serving();
    let home = daemon.home.path();
    let pane = 41;
    let asked = devpit_ptyd::pane::Asked {
        rows: 24,
        cols: 80,
        cwd: home.to_string_lossy().into_owned(),
        argv: vec!["/bin/sh".into()],
        env: Vec::new(),
    };
    crate::ptyd_link::spawn(home, pane, &asked).expect("spawn");
    let mut watching = crate::ptyd_link::connect(home).expect("connect");
    frame::write(&mut watching, &Frame::new(pane, Kind::Attach, Vec::new())).expect("attach");
    watching
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .expect("timeout");

    write_at(home, pane, &pasted("echo pane-io-$((40+2))", true)).expect("written");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut seen = Vec::new();
    while std::time::Instant::now() < deadline && !seen.windows(10).any(|w| w == b"pane-io-42") {
        match frame::read(&mut watching) {
            Ok(Some(one)) if matches!(one.kind(), Some(Kind::Output | Kind::Replay)) => {
                seen.extend(one.bytes)
            }
            Ok(Some(_)) => {}
            _ => break,
        }
    }
    crate::ptyd_link::kill(home, pane);
    assert!(
        seen.windows(10).any(|w| w == b"pane-io-42"),
        "{}",
        String::from_utf8_lossy(&seen)
    );
}

#[test]
fn a_typed_line_goes_as_it_is_then_enter() {
    assert_eq!(typed("claude --name x"), b"claude --name x\r");
}

/// A ptyd pane's settle: an idle shell named on consecutive looks is at its
/// prompt; anything else named is busy; nothing named is not known.
#[test]
fn a_ptyd_pane_is_ready_once_the_daemon_names_an_idle_shell_twice() {
    let quick = std::time::Duration::from_secs(2);
    let mut said = vec![None, Some("zsh"), Some("zsh")].into_iter();
    assert!(matches!(
        settled_on(|| said.next().flatten().map(str::to_owned), quick),
        Ready::Prompt
    ));
    let mut said = vec![Some("zsh"), Some("claude")].into_iter();
    assert!(matches!(
        settled_on(|| said.next().flatten().map(str::to_owned), quick),
        Ready::Busy(program) if program == "claude"
    ));
    assert!(matches!(
        settled_on(|| None, std::time::Duration::from_millis(300)),
        Ready::Unknown
    ));
}
