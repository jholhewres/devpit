use std::sync::mpsc::Sender;
use std::time::Instant;

use super::*;
use crate::daemon::Daemon;
use crate::pane::Asked;
use crate::socket;

/// A daemon on a socket of its own, kept at hand to look into.
fn daemon() -> (tempfile::TempDir, std::path::PathBuf, Arc<Daemon>) {
    let dir = tempfile::tempdir().expect("dir");
    let at = dir.path().join("devpit-ptyd.sock");
    let listener = socket::listen(&at).expect("listen");
    let daemon = Daemon::new();
    let serving = Arc::clone(&daemon);
    std::thread::spawn(move || serving.serve(listener));
    (dir, at, daemon)
}

/// Pane `pane` running a shell, started over a connection of its own that
/// is then dropped: nobody but the attach client is watching it.
fn shell_at(at: &Path, pane: u32) {
    let mut owner = socket::connect(at).expect("connect");
    let asked = Asked {
        rows: 24,
        cols: 80,
        cwd: std::env::temp_dir().to_string_lossy().into_owned(),
        argv: vec!["/bin/sh".into()],
        env: Vec::new(),
    };
    frame::write(&mut owner, &Frame::new(pane, Kind::Spawn, asked.encode())).expect("spawn");
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
        if matches_running(at, pane) {
            return;
        }
    }
    panic!("pane {pane} never started");
}

fn matches_running(at: &Path, pane: u32) -> bool {
    let Ok(mut probe) = socket::connect(at) else {
        return false;
    };
    let _ = probe.set_read_timeout(Some(Duration::from_millis(200)));
    let _ = frame::write(&mut probe, &Frame::new(pane, Kind::Attach, Vec::new()));
    !matches!(frame::read(&mut probe), Ok(Some(one)) if one.kind() == Some(Kind::Refused))
}

/// What the attach client wrote, readable from the test's side.
#[derive(Clone, Default)]
struct Screen(Arc<Mutex<Vec<u8>>>);

impl Write for Screen {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("screen").extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Screen {
    fn shows(&self, text: &str) -> bool {
        let seen = self.0.lock().expect("screen");
        seen.windows(text.len()).any(|w| w == text.as_bytes())
    }
    fn wait_for(&self, text: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && !self.shows(text) {
            std::thread::sleep(Duration::from_millis(20));
        }
        let seen = String::from_utf8_lossy(&self.0.lock().expect("screen")).into_owned();
        assert!(self.shows(text), "never saw {text:?} in {seen:?}");
    }
}

struct Attached {
    keys: UnixStream,
    sizes: Sender<(u16, u16)>,
    screen: Screen,
    ended: std::thread::JoinHandle<io::Result<Ended>>,
}

fn attached(at: &Path, pane: u32) -> Attached {
    let stream = socket::connect(at).expect("connect");
    let (keys, input) = UnixStream::pair().expect("pair");
    let (sizes, sized) = channel();
    let screen = Screen::default();
    let mut output = screen.clone();
    let ended = std::thread::spawn(move || serve(stream, pane, None, input, &mut output, sized));
    Attached {
        keys,
        sizes,
        screen,
        ended,
    }
}

#[test]
fn only_output_is_owed_never_the_replay() {
    let output = Frame::new(3, Kind::Output, vec![b'x'; 70]);
    let replay = Frame::new(3, Kind::Replay, vec![b'x'; 70]);
    assert_eq!(owed(&output), Some(70));
    assert_eq!(owed(&replay), None);
    assert_eq!(owed(&Frame::new(3, Kind::Cwd, b"/tmp".to_vec())), None);
}

#[test]
fn under_tmux_the_pane_is_sized_only_when_tmux_resizes_it() {
    assert!(!sized_at_start(true));
    assert!(sized_at_start(false));
}

/// Keys typed reach the shell, what it prints reaches the terminal, and
/// with no other watcher the daemon is left owing nothing once it is quiet:
/// a pane the attach client alone watches never stops for want of acks.
#[test]
fn typing_reaches_the_pane_its_output_reaches_the_terminal_and_is_acked() {
    let (_dir, at, daemon) = daemon();
    shell_at(&at, 21);
    let mut one = attached(&at, 21);
    one.keys
        .write_all(b"echo attach-$((40+2)); head -c 300000 /dev/zero | tr '\\0' x; echo; echo done-$((1+1))\n")
        .expect("type");
    one.screen.wait_for("attach-42");
    one.screen.wait_for("done-2");
    let pane = daemon.pane(21).expect("pane");
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && pane.unacked() != 0 {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(pane.unacked(), 0, "output was left unacknowledged");

    one.sizes.send((33, 101)).expect("size");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !one.screen.shows("33 101") {
        one.keys.write_all(b"stty size\n").expect("type");
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(one.screen.shows("33 101"), "the pane was never resized");

    drop(one.keys);
    let ended = one.ended.join().expect("join").expect("served");
    assert_eq!(ended, Ended::Left, "a closed terminal leaves the pane");
    assert!(matches_running(&at, 21), "leaving does not end the pane");
}

/// Beside a desktop window the attach client leaves the acking to it: the
/// pane is held for the window, however much is read and acked here.
#[test]
fn beside_a_window_the_attach_client_leaves_the_acking_to_it() {
    let (_dir, at, daemon) = daemon();
    shell_at(&at, 23);
    let mut window = socket::connect(&at).expect("connect");
    frame::write(&mut window, &Frame::new(23, Kind::Attach, Vec::new())).expect("attach");
    let mut one = attached(&at, 23);
    one.keys.write_all(b"yes mirrored\n").expect("type");
    one.screen.wait_for("mirrored\r\nmirrored");
    std::thread::sleep(Duration::from_secs(1));
    let pane = daemon.pane(23).expect("pane");
    let held = pane.unacked();
    pane.kill();
    assert!(
        held > crate::pane::MOST_UNACKED,
        "the attach client acked what the window owed: {held} held"
    );
}

#[test]
fn the_pane_ending_ends_the_attach_with_its_code() {
    let (_dir, at, _daemon) = daemon();
    shell_at(&at, 22);
    let mut one = attached(&at, 22);
    one.keys.write_all(b"exit 7\n").expect("type");
    let ended = one.ended.join().expect("join").expect("served");
    assert_eq!(ended, Ended::Exited(Some(7)));
}

#[test]
fn a_pane_that_is_not_there_is_refused() {
    let (_dir, at, _daemon) = daemon();
    let one = attached(&at, 404);
    let ended = one.ended.join().expect("join").expect("served");
    assert!(matches!(ended, Ended::Refused(_)), "{ended:?}");
}
