use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use super::*;
use crate::ptyd_link::pane_number;

fn shell(home: &std::path::Path) -> devpit_ptyd::pane::Asked {
    devpit_ptyd::pane::Asked {
        rows: 24,
        cols: 80,
        cwd: home.to_string_lossy().into_owned(),
        argv: vec!["/bin/sh".into()],
        env: Vec::new(),
    }
}

/// What reached the viewer so far: the bytes, decoded, and whether the pane
/// was said to be closed. Every message is checked to be one a tmux pane's
/// viewer already reads, named for the leaf.
fn heard_until(
    heard: &Receiver<RemoteOut>,
    leaf: &str,
    seen: &mut String,
    done: impl Fn(&str, bool) -> bool,
) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut closed = false;
    while Instant::now() < deadline && !done(seen, closed) {
        let Ok(one) = heard.recv_timeout(Duration::from_millis(100)) else {
            continue;
        };
        let wire = serde_json::to_string(&one).expect("json");
        match one {
            RemoteOut::PaneBytes { pane, b64 } => {
                assert_eq!(pane, leaf);
                assert!(
                    wire.starts_with(&format!(r#"{{"t":"paneBytes","pane":"{leaf}","b64":""#)),
                    "{wire}"
                );
                seen.push_str(&String::from_utf8_lossy(&BASE64.decode(b64).expect("b64")));
            }
            RemoteOut::PaneClosed { pane } => {
                assert_eq!(pane, leaf);
                assert_eq!(wire, format!(r#"{{"t":"paneClosed","pane":"{leaf}"}}"#));
                closed = true;
            }
            other => panic!("not a pane's: {other:?}"),
        }
    }
    done(seen, closed)
}

/// A ptyd pane watched from a phone: its bytes come as a tmux pane's do,
/// keys typed there reach it, its size stays the desk's whatever the phone's
/// is, and closing it says so.
#[test]
fn a_ptyd_panes_bytes_reach_a_remote_watcher_in_the_shapes_a_tmux_panes_do() {
    let daemon = crate::ptyd_link::tests::serving();
    let home = daemon.home.path();
    let leaf = "leaf_remote_watch";
    let number = pane_number(leaf);
    crate::ptyd_link::spawn(home, number, &shell(home)).expect("spawn");
    let (out, heard) = channel();
    let mut watching = Watching::new(7);
    watching
        .open_ptyd_at(home, leaf, number, Sink::Json(out.clone()), out)
        .expect("watching");

    watching
        .input(leaf, b"echo remote-$((40+2)); stty size\n")
        .expect("typed");
    let mut seen = String::new();
    let shown = heard_until(&heard, leaf, &mut seen, |seen, _| {
        seen.contains("remote-42") && seen.contains("24 80")
    });
    assert!(shown, "never shown: {seen:?}");

    watching.close(leaf);
    assert!(
        heard_until(&heard, leaf, &mut seen, |_, closed| closed),
        "closing it was never said"
    );
    crate::ptyd_link::kill(home, number);
}

#[test]
fn a_ptyd_pane_that_ends_is_closed_for_its_watcher() {
    let daemon = crate::ptyd_link::tests::serving();
    let home = daemon.home.path();
    let leaf = "leaf_remote_ends";
    let number = pane_number(leaf);
    crate::ptyd_link::spawn(home, number, &shell(home)).expect("spawn");
    let (out, heard) = channel();
    let mut watching = Watching::new(8);
    watching
        .open_ptyd_at(home, leaf, number, Sink::Json(out.clone()), out)
        .expect("watching");
    watching.input(leaf, b"exit\n").expect("typed");
    let mut seen = String::new();
    assert!(
        heard_until(&heard, leaf, &mut seen, |_, closed| closed),
        "its end was never said"
    );
}

/// A viewer that gave the pane a handle reads its bytes raw under it, never
/// as base64, and still hears the pane close in JSON.
#[test]
fn a_ptyd_panes_bytes_reach_a_viewer_that_reads_raw_under_its_handle() {
    let daemon = crate::ptyd_link::tests::serving();
    let home = daemon.home.path();
    let leaf = "leaf_remote_raw";
    let number = pane_number(leaf);
    crate::ptyd_link::spawn(home, number, &shell(home)).expect("spawn");
    let (out, heard) = channel();
    let (raw, bytes) = channel();
    let mut watching = Watching::new(9);
    watching.raw = Some(raw);
    let sink = watching.sink(&out, Some(41));
    watching
        .open_ptyd_at(home, leaf, number, sink, out)
        .expect("watching");
    watching
        .input(leaf, b"echo raw-$((40+2))\n")
        .expect("typed");
    let mut seen = Vec::new();
    let until = Instant::now() + Duration::from_secs(10);
    while !String::from_utf8_lossy(&seen).contains("raw-42") && Instant::now() < until {
        if let Ok(one) = bytes.recv_timeout(Duration::from_millis(100)) {
            let one: Raw = one;
            assert_eq!(one.handle, 41);
            seen.extend(one.bytes);
        }
    }
    assert!(
        String::from_utf8_lossy(&seen).contains("raw-42"),
        "never shown: {seen:?}"
    );
    assert!(
        heard
            .try_iter()
            .all(|said| !matches!(said, RemoteOut::PaneBytes { .. })),
        "bytes went out as base64 too"
    );
    watching.close(leaf);
    let mut text = String::new();
    assert!(
        heard_until(&heard, leaf, &mut text, |_, closed| closed),
        "closing it was never said"
    );
    crate::ptyd_link::kill(home, number);
}

#[test]
fn a_handle_without_a_raw_connection_keeps_the_json_shape() {
    let (out, _heard) = channel();
    let watching = Watching::new(10);
    assert!(matches!(watching.sink(&out, Some(3)), Sink::Json(_)));
}
