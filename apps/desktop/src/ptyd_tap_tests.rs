use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use devpit_pty::blocks::{Cut, Head, Segmenter};
use devpit_pty::Told;

use super::*;

/// Ears that cut what they hear into blocks as the app's do, on a clock that
/// moves eleven seconds a chunk: a command that starts and ends in different
/// chunks is a long one.
#[derive(Default)]
struct Heard {
    segmenter: Mutex<Segmenter>,
    clock: AtomicU64,
    bytes: AtomicUsize,
    told: Mutex<Vec<Told>>,
    ended: Mutex<Vec<Head>>,
    window: AtomicBool,
}

impl Ears for Heard {
    fn heard(&self, _project: &str, _leaf: &str, bytes: &[u8]) {
        self.bytes.fetch_add(bytes.len(), Ordering::SeqCst);
        let now = self.clock.fetch_add(11_000, Ordering::SeqCst);
        let mut told = Vec::new();
        let mut ended = Vec::new();
        self.segmenter.lock().expect("segmenter").feed(
            bytes,
            now,
            |one| told.push(one),
            |cut, _| {
                if let Cut::Ended(head) = cut {
                    ended.push(head);
                }
            },
        );
        self.told.lock().expect("told").extend(told);
        self.ended.lock().expect("ended").extend(ended);
    }

    fn window_on(&self, _leaf: &str) -> bool {
        self.window.load(Ordering::SeqCst)
    }

    fn changed(&self, _project: &str) {}

    fn places(&self) -> HashMap<u32, (String, String)> {
        HashMap::new()
    }
}

fn until(mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// `tap`'s session with the daemon of `home`, on a thread, once connected.
fn listening(tap: &Arc<Tap>, ears: &Arc<Heard>, home: &Path) {
    let socket = crate::ptyd_link::socket_path(home);
    let (tap_, ears_) = (Arc::clone(tap), Arc::clone(ears));
    std::thread::spawn(move || tap_.session(&socket, &*ears_));
    assert!(until(|| tap.link.lock().expect("link").is_some()));
}

fn hang_up(tap: &Tap) {
    if let Some(link) = tap.link.lock().expect("link").as_ref() {
        let _ = link.stream.shutdown(std::net::Shutdown::Both);
    }
}

fn sh(home: &Path, script: &str) -> devpit_ptyd::pane::Asked {
    devpit_ptyd::pane::Asked {
        rows: 24,
        cols: 80,
        cwd: home.to_string_lossy().into_owned(),
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        env: Vec::new(),
    }
}

#[test]
fn the_tap_acks_only_while_no_window_is_on_the_pane() {
    assert_eq!(acked(false, 4096), Some(4096));
    assert_eq!(acked(true, 4096), None);
    assert_eq!(acked(false, usize::MAX), Some(u32::MAX));
}

/// No window anywhere: what the pane runs is heard by the tap alone, cut
/// into a block, and a long one is worth the bell.
#[test]
fn a_ptyd_pane_with_no_window_is_cut_into_blocks_and_a_long_command_is_told() {
    let daemon = crate::ptyd_link::tests::serving();
    let home = daemon.home.path();
    let (tap, ears) = (Arc::new(Tap::default()), Arc::new(Heard::default()));
    listening(&tap, &ears, home);

    let leaf = "leaf_tap_blocks";
    let pane = pane_number(leaf);
    let script = r#"while read x; do printf '\033]133;A\007\033]777;devpit-cmd;make build\007\033]133;C\007'; sleep 0.3; printf 'built\r\n\033]133;D;0\007'; done"#;
    crate::ptyd_link::spawn(home, pane, &sh(home, script)).expect("spawn");
    tap.heed("p", leaf);
    let ended = until(|| {
        let _ = crate::pane_io::write_at(home, pane, b"\n");
        !ears.ended.lock().expect("ended").is_empty()
    });
    crate::ptyd_link::kill(home, pane);
    hang_up(&tap);

    assert!(ended, "no block ended");
    let head = ears.ended.lock().expect("ended")[0].clone();
    assert_eq!(head.command.as_deref(), Some("make build"));
    assert_eq!(head.code, Some(0));
    assert!(crate::blocks::worth_telling(&head));
    let told = ears.told.lock().expect("told");
    assert!(told
        .iter()
        .any(|one| matches!(one, Told::CommandEnded { code: Some(0) })));
}

/// The split's folder is `pane_io::cwd`, and for a ptyd pane that is what
/// the shell last told the daemon, heard by the app's tap.
#[test]
fn the_split_of_a_ptyd_pane_starts_where_its_shell_said_it_stands() {
    let daemon = crate::ptyd_link::tests::serving();
    let home = daemon.home.path();
    let sub = home.join("sub");
    std::fs::create_dir(&sub).expect("sub");
    let ears = Arc::new(Heard::default());
    let socket = crate::ptyd_link::socket_path(home);
    let listening = Arc::clone(&ears);
    std::thread::spawn(move || tap().session(&socket, &*listening));

    let leaf = "leaf_tap_cwd";
    let pane = pane_number(leaf);
    let script = r#"cd sub && printf '\033]7;file://localhost%s\007' "$PWD"; sleep 30"#;
    crate::ptyd_link::spawn(home, pane, &sh(home, script)).expect("spawn");
    let target = crate::ptyd_link::target(leaf);
    let told = until(|| crate::pane_io::cwd(&target).as_deref() == Some(sub.as_path()));
    crate::ptyd_link::kill(home, pane);
    hang_up(tap());
    assert!(told, "{:?}", crate::pane_io::cwd(&target));
}

/// A daemon of the test's own, to ask what it holds: the socket, and the
/// daemon itself.
fn own_daemon(home: &Path) -> (PathBuf, Arc<devpit_ptyd::daemon::Daemon>) {
    let socket = crate::ptyd_link::socket_path(home);
    let listener = devpit_ptyd::socket::listen(&socket).expect("listen");
    let daemon = devpit_ptyd::daemon::Daemon::new();
    let serving = Arc::clone(&daemon);
    std::thread::spawn(move || serving.serve(listener));
    (socket, daemon)
}

/// Drawn before the tap was there: all it has is the replay, which leaves
/// the program in the alternate screen with its arrows in application mode.
#[test]
fn a_screen_seeded_from_a_replay_is_the_alternate_screen_the_program_left_it_in() {
    let home = tempfile::tempdir().expect("home");
    let home = home.path();
    let (socket, daemon) = own_daemon(home);
    let leaf = "leaf_tap_alt";
    let pane = pane_number(leaf);
    let script = r#"printf 'on the primary
[?1049h[?1h[Hin the alt screen'; sleep 30"#;
    crate::ptyd_link::spawn(home, pane, &sh(home, script)).expect("spawn");
    let printed = || {
        let held = daemon
            .pane(pane)
            .map(|one| one.older(0))
            .unwrap_or_default();
        held.windows(10).any(|w| w == b"alt screen")
    };
    assert!(until(printed), "the pane never printed");

    let (tap, ears) = (Arc::new(Tap::default()), Arc::new(Heard::default()));
    listening(&tap, &ears, home);
    tap.heed("p", leaf);
    let drawn = until(|| {
        tap.screen(pane)
            .is_some_and(|shown| shown.contains("in the alt screen"))
    });
    crate::ptyd_link::kill(home, pane);
    hang_up(&tap);
    let _ = std::fs::remove_file(&socket);

    assert!(drawn, "{:?}", tap.screen(pane));
    let shown = tap.screen(pane).unwrap_or_default();
    assert!(!shown.contains("on the primary"), "{shown}");
    assert!(tap
        .modes(pane)
        .is_some_and(|modes| modes.application_cursor));
    assert_eq!(
        ears.bytes.load(Ordering::SeqCst),
        0,
        "a replay is not heard"
    );
}

/// The question an agent is stopped on, read off a ptyd pane's screen as it
/// is off tmux's capture.
#[test]
fn the_question_on_a_ptyd_pane_is_read_off_its_screen() {
    let daemon = crate::ptyd_link::tests::serving();
    let home = daemon.home.path();
    let asked = " Bash command\r\n   git push origin main\r\n Do you want to proceed?\r\n   1. Yes\r\n \u{276f} 2. Yes, and don't ask again\r\n   3. No (esc)\r\n";
    std::fs::write(home.join("asked"), asked).expect("asked");
    let (tap, ears) = (Arc::new(Tap::default()), Arc::new(Heard::default()));
    listening(&tap, &ears, home);
    let leaf = "leaf_tap_prompt";
    let pane = pane_number(leaf);
    crate::ptyd_link::spawn(home, pane, &sh(home, "sleep 0.3; cat asked; sleep 30"))
        .expect("spawn");
    tap.heed("p", leaf);
    let read = || {
        tap.screen(pane)
            .as_deref()
            .and_then(crate::live_prompt::pending)
    };
    let found = until(|| read().is_some());
    crate::ptyd_link::kill(home, pane);
    hang_up(&tap);

    assert!(found, "{:?}", tap.screen(pane));
    let prompt = read().expect("a question");
    assert_eq!(
        prompt.question,
        "Bash command git push origin main Do you want to proceed?"
    );
    assert_eq!(prompt.options.len(), 3);
    assert_eq!(prompt.cursor, 1);
}

/// The count the daemon keeps per pane: the tap takes from it alone while no
/// window is on the pane, leaves it to the window while one is, and clears
/// what the window left when it goes.
#[test]
fn the_tap_acks_a_pane_no_window_is_on_and_takes_over_when_the_window_goes() {
    let home = tempfile::tempdir().expect("home");
    let home = home.path();
    let (socket, daemon) = own_daemon(home);
    let (tap, ears) = (Arc::new(Tap::default()), Arc::new(Heard::default()));
    listening(&tap, &ears, home);

    let leaf = "leaf_tap_acks";
    let pane = pane_number(leaf);
    crate::ptyd_link::spawn(home, pane, &sh(home, "exec /bin/sh")).expect("spawn");
    tap.heed("p", leaf);
    let unacked = || daemon.pane(pane).map(|one| one.unacked()).unwrap_or(0);
    let flood = b"yes | head -c 300000\n";

    // Typed until heard: the tap's attach may land after the first line.
    assert!(until(|| {
        let _ = crate::pane_io::write_at(home, pane, flood);
        ears.bytes.load(Ordering::SeqCst) >= 300_000
    }));
    assert!(until(|| unacked() == 0), "no window: the tap acks");

    let mut window = devpit_ptyd::socket::connect(&socket).expect("window");
    frame::write(&mut window, &Frame::new(pane, Kind::Attach, Vec::new())).expect("attach");
    let replayed = frame::read(&mut window).expect("replay").expect("a frame");
    assert_eq!(replayed.kind(), Some(Kind::Replay));
    ears.window.store(true, Ordering::SeqCst);
    crate::pane_io::write_at(home, pane, flood).expect("typed");
    assert!(
        until(|| unacked() >= 300_000),
        "a window that has not acked holds the pane: {}",
        unacked()
    );

    let _ = window.shutdown(std::net::Shutdown::Both);
    ears.window.store(false, Ordering::SeqCst);
    tap.let_go(leaf, ears.window_on(leaf));
    assert!(until(|| unacked() == 0), "{}", unacked());

    crate::ptyd_link::kill(home, pane);
    hang_up(&tap);
    let _ = std::fs::remove_file(&socket);
}

/// The desktop killed with `kill -9`: every connection it had closes with
/// it and everything it knew is gone, but the daemon and the agent are not.
/// Started again, it finds the pane running and starts nothing in its place,
/// and its tap reads the agent's alternate screen back off the replay.
#[test]
fn an_agent_outlives_the_desktop_killed_and_comes_back_in_its_alternate_screen() {
    use devpit_ptyd::frame::{self, Frame, Kind};

    let home = tempfile::tempdir().expect("home");
    let home = home.path();
    let (socket, daemon) = own_daemon(home);
    let leaf = format!("leaf_kill9_{}", std::process::id());
    let pane = pane_number(&leaf);
    let script =
        r#"printf 'on the primary\r\n\033[?1049h\033[?1h\033[Hhello from the agent'; sleep 30"#;
    let mut asked = sh(home, script);
    asked.env = vec![(devpit_tmux::PANE_ENV.to_owned(), leaf.clone())];
    crate::ptyd_link::spawn(home, pane, &asked).expect("spawn");

    // The desktop before: its tap on the pane, and a window attached to it.
    let (tap, ears) = (Arc::new(Tap::default()), Arc::new(Heard::default()));
    listening(&tap, &ears, home);
    tap.heed("p", &leaf);
    let shows = |tap: &Tap| {
        tap.screen(pane)
            .is_some_and(|shown| shown.contains("hello from the agent"))
    };
    assert!(until(|| shows(&tap)), "{:?}", tap.screen(pane));
    let mut window = crate::ptyd_link::connect(home).expect("connect");
    frame::write(&mut window, &Frame::new(pane, Kind::Attach, Vec::new())).expect("attach");

    // `kill -9`: the process's sockets close with it, and its state is gone.
    hang_up(&tap);
    drop((tap, ears, window));

    let store = devpit_core::Store::open(&home.join("state.db")).expect("store");
    let project = store.add_project(home, None).expect("project");
    let revived = crate::ptyd_link::revive_at(&store, Some(home), &project, &leaf, None, home);
    let (tap, ears) = (Arc::new(Tap::default()), Arc::new(Heard::default()));
    listening(&tap, &ears, home);
    tap.heed(&project, &leaf);
    let drawn = until(|| shows(&tap));
    let shown = tap.screen(pane).unwrap_or_default();
    let modes = tap.modes(pane);
    #[cfg(target_os = "linux")]
    let alive = devpit_pty::started_with(devpit_tmux::PANE_ENV, &leaf)
        .is_some_and(|found| !found.is_empty());
    #[cfg(not(target_os = "linux"))]
    let alive = crate::ptyd_link::running(home, pane);
    crate::ptyd_link::kill(home, pane);
    hang_up(&tap);
    let _ = std::fs::remove_file(&socket);

    assert!(!revived, "a running pane was started again");
    assert!(daemon.pane(pane).is_some());
    assert!(alive, "the agent did not outlive the desktop");
    assert!(drawn, "{shown}");
    assert!(!shown.contains("on the primary"), "{shown}");
    assert!(modes.is_some_and(|modes| modes.application_cursor));
}
