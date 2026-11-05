use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use super::*;

fn asked(script: &str) -> Asked {
    Asked {
        rows: 24,
        cols: 80,
        cwd: std::env::temp_dir().to_string_lossy().into_owned(),
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        env: vec![("DEVPIT_PANE".into(), "leaf_x".into())],
    }
}

/// Frames until `until` holds or `wait` passes; returns all of them.
fn gather(queue: &Receiver<Frame>, wait: Duration, until: impl Fn(&[Frame]) -> bool) -> Vec<Frame> {
    let deadline = Instant::now() + wait;
    let mut got = Vec::new();
    while Instant::now() < deadline && !until(&got) {
        if let Ok(one) = queue.recv_timeout(Duration::from_millis(50)) {
            got.push(one);
        }
    }
    got
}

fn printed(frames: &[Frame]) -> Vec<u8> {
    frames
        .iter()
        .filter(|f| matches!(f.kind(), Some(Kind::Output | Kind::Replay)))
        .flat_map(|f| f.bytes.clone())
        .collect()
}

#[test]
fn a_spawn_frame_reads_back_as_written_and_a_bad_one_is_none() {
    let one = asked("echo hi");
    assert_eq!(Asked::parse(&one.encode()), Some(one));
    assert_eq!(Asked::parse(&[24, 0]), None);
    let no_command = Asked {
        argv: Vec::new(),
        ..asked("x")
    };
    assert_eq!(Asked::parse(&no_command.encode()), None);
    let no_size = Asked {
        rows: 0,
        ..asked("x")
    };
    assert_eq!(Asked::parse(&no_size.encode()), None);
}

#[test]
fn a_late_client_is_put_back_in_the_modes_then_shown_the_tail() {
    let pane = Pane::spawn(
        1,
        &asked("printf '\\033[?1049hscreen-one'; sleep 5"),
        |_| {},
    )
    .expect("spawn");
    let (early, early_queue) = crate::client::Client::queued();
    pane.attach(&early);
    gather(&early_queue, Duration::from_secs(5), |got| {
        printed(got).windows(10).any(|w| w == b"screen-one")
    });
    let (late, late_queue) = crate::client::Client::queued();
    pane.attach(&late);
    let replay: Vec<Frame> = gather(&late_queue, Duration::from_millis(300), |_| false)
        .into_iter()
        .filter(|f| f.kind() == Some(Kind::Replay))
        .collect();
    assert_eq!(
        replay.first().map(|f| f.bytes.clone()),
        Some(b"\x1b[?1049h".to_vec())
    );
    assert!(
        printed(&replay).ends_with(b"screen-one"),
        "{:?}",
        printed(&replay)
    );
    pane.kill();
}

#[test]
fn the_end_is_told_with_its_code() {
    let (client, queue) = crate::client::Client::queued();
    let (gone_tx, gone) = std::sync::mpsc::channel();
    let pane = Pane::spawn(2, &asked("exit 3"), move |id| {
        let _ = gone_tx.send(id);
    })
    .expect("spawn");
    pane.attach(&client);
    let got = gather(&queue, Duration::from_secs(5), |got| {
        got.iter().any(|f| f.kind() == Some(Kind::Exited))
    });
    let exited = got
        .iter()
        .find(|f| f.kind() == Some(Kind::Exited))
        .expect("exited");
    assert_eq!(exited.bytes, 3i32.to_le_bytes());
    assert_eq!(
        gone.recv_timeout(Duration::from_secs(5))
            .expect("forgotten"),
        2
    );
}

#[test]
fn a_cd_is_told_as_the_panes_cwd() {
    let (client, queue) = crate::client::Client::queued();
    let pane = Pane::spawn(
        3,
        &asked("printf '\\033]7;file://host/tmp/ptyd-here\\007'; sleep 5"),
        |_| {},
    )
    .expect("spawn");
    pane.attach(&client);
    let got = gather(&queue, Duration::from_secs(5), |got| {
        got.iter().any(|f| f.kind() == Some(Kind::Cwd))
    });
    let cwd = got
        .iter()
        .find(|f| f.kind() == Some(Kind::Cwd))
        .expect("cwd");
    assert_eq!(cwd.bytes, b"/tmp/ptyd-here");
    assert_eq!(pane.cwd().as_deref(), Some("/tmp/ptyd-here"));
    pane.kill();
}

#[test]
fn the_foreground_program_is_told_once_the_output_settles() {
    let (client, queue) = crate::client::Client::queued();
    let pane = Pane::spawn(4, &asked("echo start; exec sleep 5"), |_| {}).expect("spawn");
    pane.attach(&client);
    let got = gather(&queue, Duration::from_secs(5), |got| {
        got.iter()
            .any(|f| f.kind() == Some(Kind::Program) && f.bytes == b"sleep")
    });
    assert!(
        got.iter()
            .any(|f| f.kind() == Some(Kind::Program) && f.bytes == b"sleep"),
        "{:?}",
        got.iter()
            .filter(|f| f.kind() == Some(Kind::Program))
            .collect::<Vec<_>>()
    );
    pane.kill();
}

/// A window that attaches after the program settled is told it at once,
/// as it is told the cwd: no change is coming to say it again.
#[test]
fn a_late_attach_is_told_the_foreground_program() {
    let (first, first_queue) = crate::client::Client::queued();
    let pane = Pane::spawn(9, &asked("echo start; exec sleep 5"), |_| {}).expect("spawn");
    pane.attach(&first);
    gather(&first_queue, Duration::from_secs(5), |got| {
        got.iter().any(|f| f.kind() == Some(Kind::Program))
    });
    let (late, late_queue) = crate::client::Client::queued();
    pane.attach(&late);
    let got = gather(&late_queue, Duration::from_millis(500), |got| {
        got.iter().any(|f| f.kind() == Some(Kind::Program))
    });
    pane.kill();
    assert!(
        got.iter()
            .any(|f| f.kind() == Some(Kind::Program) && f.bytes == b"sleep"),
        "{:?}",
        got.iter().map(Frame::kind).collect::<Vec<_>>()
    );
}

/// Past a megabyte unacknowledged the pty is not read: what is buffered stops
/// growing however long the program writes, and an ack lets it read on.
#[test]
fn unacknowledged_output_stops_the_reading_and_an_ack_resumes_it() {
    let (client, queue) = crate::client::Client::queued();
    let pane = Pane::spawn(5, &asked("yes 0123456789abcdef"), |_| {}).expect("spawn");
    pane.attach(&client);
    std::thread::sleep(Duration::from_millis(800));
    let held = pane.unacked();
    assert!(held > MOST_UNACKED, "{held}");
    assert!(
        held <= MOST_UNACKED + SLICE,
        "stopped within a slice: {held}"
    );
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        pane.unacked(),
        held,
        "nothing more read while unacknowledged"
    );
    while queue.try_recv().is_ok() {}
    pane.ack(held);
    std::thread::sleep(Duration::from_millis(300));
    assert!(pane.unacked() > 0, "reading again after the ack");
    pane.kill();
}

/// 500 MB through a client that acknowledges what it reads: memory is held
/// to the window, and Ctrl+C stops the flood well within a second.
#[test]
fn a_flood_of_500_mb_stays_bounded_and_ctrl_c_stops_it() {
    let (client, queue) = crate::client::Client::queued();
    let pane = Pane::spawn(
        6,
        &asked("head -c 524288000 /dev/zero | tr '\\0' 'x'; sleep 30"),
        |_| {},
    )
    .expect("spawn");
    pane.attach(&client);
    let mut seen = 0usize;
    let mut most_held = 0usize;
    let started = Instant::now();
    while seen < 64 * 1024 * 1024 && started.elapsed() < Duration::from_secs(60) {
        if let Ok(one) = queue.recv_timeout(Duration::from_millis(100)) {
            if one.kind() == Some(Kind::Output) {
                seen += one.bytes.len();
                pane.ack(one.bytes.len());
            }
        }
        most_held = most_held.max(pane.unacked());
    }
    assert!(seen >= 64 * 1024 * 1024, "the flood flowed: {seen}");
    assert!(
        most_held <= MOST_UNACKED + SLICE,
        "held to the window: {most_held}"
    );

    pane.write(&[0x03]).expect("ctrl+c");
    let asked_at = Instant::now();
    let mut quiet_since = Instant::now();
    while asked_at.elapsed() < Duration::from_secs(3) {
        match queue.recv_timeout(Duration::from_millis(20)) {
            Ok(one) => {
                if one.kind() == Some(Kind::Output) {
                    pane.ack(one.bytes.len());
                    if one.bytes.contains(&b'x') {
                        quiet_since = Instant::now();
                    }
                }
            }
            Err(_) if quiet_since.elapsed() > Duration::from_millis(300) => break,
            Err(_) => {}
        }
    }
    let stopped_after = quiet_since.duration_since(asked_at);
    assert!(
        stopped_after < Duration::from_secs(1),
        "ctrl+c took {stopped_after:?}"
    );
    pane.kill();
}

#[cfg(target_os = "linux")]
#[test]
fn a_killed_pane_ends_even_when_what_it_started_ignores_the_hangup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let started = dir.path().join("started");
    let script = format!(
        "sh -c \"trap '' HUP; touch '{}'; exec sleep 300\" & wait",
        started.display()
    );
    let (gone_tx, gone) = std::sync::mpsc::channel();
    let pane = Pane::spawn(9, &asked(&script), move |id| {
        let _ = gone_tx.send(id);
    })
    .expect("spawn");
    let deadline = Instant::now() + Duration::from_secs(30);
    while !started.exists() {
        assert!(Instant::now() < deadline, "the child never started");
        std::thread::sleep(Duration::from_millis(20));
    }

    pane.kill();

    assert_eq!(gone.recv_timeout(ENDS_WITHIN).expect("the pane ended"), 9);
}
