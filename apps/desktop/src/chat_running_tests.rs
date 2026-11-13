//! What a turn claims, and what it must let go of.

use super::*;

/// A failed turn used to leave its pid in the map: the `?` at the end of
/// `chat_send` returned before the cleanup below it. The next `chat.cancel`
/// then sent SIGTERM to a pid the system had reused.
#[test]
fn a_failed_turn_leaves_no_pid_behind() {
    let talking = Talking::default();
    let steering = Steering::default();

    {
        let guard = talking.begin("conv_1", &steering).expect("the first turn");
        // What the turn's own callback writes when the CLI starts.
        talking
            .running
            .lock()
            .expect("running")
            .insert("conv_1".to_owned(), Some(4242));
        assert_eq!(
            talking.running.lock().expect("running").get("conv_1"),
            Some(&Some(4242))
        );
        // The turn fails here, without reaching any cleanup of its own: this
        // is the `?` at the end of `chat_send`, which used to return before
        // the two lines that let go.
        drop(guard);
    }

    assert!(
        talking.running.lock().expect("running").is_empty(),
        "the pid outlived the turn"
    );
    assert!(
        !steering.holds("conv_1"),
        "the steering handle outlived the turn"
    );
}

/// A second send while one is running is refused rather than served: two
/// processes appending to one transcript, with one pid recorded for both, is
/// not two turns.
#[test]
fn a_second_turn_in_the_same_conversation_is_refused() {
    let talking = Talking::default();
    let steering = Steering::default();

    let first = talking.begin("conv_1", &steering).expect("the first turn");
    let Err(refused) = talking.begin("conv_1", &steering) else {
        panic!("a second turn in the same conversation was allowed");
    };
    assert_eq!(refused.code, ErrorCode::Conflict);

    // Another conversation is not this one.
    let other = talking
        .begin("conv_2", &steering)
        .expect("another conversation");

    drop(first);
    // And once the first ends, the conversation takes a turn again.
    let again = talking
        .begin("conv_1", &steering)
        .expect("after the first ended");
    drop(again);
    drop(other);
}

fn ended_within(child: &mut std::process::Child, wait: std::time::Duration) -> bool {
    let until = std::time::Instant::now() + wait;
    while std::time::Instant::now() < until {
        if child.try_wait().ok().flatten().is_some() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    false
}

/// A Stop pressed while the turn was still starting reached nothing: there
/// was no pid to signal yet, and the turn ran on. It is kept and carried out
/// as the process starts.
#[cfg(unix)]
#[test]
fn a_stop_before_the_process_starts_stops_it_as_it_starts() {
    let talking = Talking::default();
    let steering = Steering::default();
    let _guard = talking.begin("conv_1", &steering).expect("the turn");
    assert!(talking.stop("conv_1", Signal::Term));

    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("sleep");
    talking.on_start("conv_1", false)(child.id());
    let ended = ended_within(&mut child, std::time::Duration::from_secs(5));
    let _ = child.kill();
    let _ = child.wait();
    assert!(ended, "the process outlived the stop asked before it");
}

/// What "Force stop" is for: a turn whose process does not end on the polite
/// signal still ends. It leads its own group, as a turn's CLI does.
#[cfg(unix)]
#[test]
fn a_forced_stop_ends_a_process_the_polite_one_did_not() {
    use std::os::unix::process::CommandExt as _;
    let talking = Talking::default();
    let steering = Steering::default();
    let _guard = talking.begin("conv_1", &steering).expect("the turn");
    let mut child = std::process::Command::new("sh")
        .args(["-c", "trap '' TERM; echo deaf; exec sleep 30"])
        .stdout(std::process::Stdio::piped())
        .process_group(0)
        .spawn()
        .expect("sh");
    // Signalled before its trap is set, it would end on the polite one.
    let mut said = String::new();
    std::io::BufRead::read_line(
        &mut std::io::BufReader::new(child.stdout.take().expect("stdout")),
        &mut said,
    )
    .expect("deaf");
    talking.on_start("conv_1", false)(child.id());
    assert!(talking.stop("conv_1", Signal::Term));
    assert!(!ended_within(
        &mut child,
        std::time::Duration::from_millis(300)
    ));

    assert!(talking.stop("conv_1", Signal::Kill));
    let ended = ended_within(&mut child, std::time::Duration::from_secs(5));
    let _ = child.kill();
    let _ = child.wait();
    assert!(ended, "the forced stop did not end it");
}

#[test]
fn nothing_is_stopped_when_no_turn_runs() {
    let talking = Talking::default();
    assert!(!talking.stop("conv_1", Signal::Kill));
}

/// A conversation whose process stays is told before the person's words go
/// in, after the process exists: a stop then is the turn's to carry out, by
/// interrupting itself, and the process that stays is not signalled.
#[cfg(unix)]
#[test]
fn a_stop_while_a_resident_turn_starts_is_left_to_the_turn() {
    let talking = Talking::default();
    let steering = Steering::default();
    let _guard = talking.begin("conv_1", &steering).expect("the turn");
    assert!(talking.stop_starting("conv_1"));

    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("sleep");
    let goes_on = talking.on_start("conv_1", true)(child.id());
    let signalled = ended_within(&mut child, std::time::Duration::from_millis(300));
    let _ = child.kill();
    let _ = child.wait();
    assert!(!goes_on, "the turn was not told to stop");
    assert!(!signalled, "the process that stays was signalled");
    assert!(
        !talking.stop_starting("conv_1"),
        "a started turn is not starting"
    );
}

#[test]
fn only_a_turn_still_starting_is_stopped_as_it_starts() {
    let talking = Talking::default();
    let steering = Steering::default();
    assert!(!talking.stop_starting("conv_1"));
    let _guard = talking.begin("conv_1", &steering).expect("the turn");
    talking.on_start("conv_1", true)(4242);
    assert!(!talking.stop_starting("conv_1"));
}

/// Force stop is for a hung tool: it ends the turn's process group, the
/// tools the CLI started with it, not the CLI alone.
#[cfg(unix)]
#[test]
fn a_forced_stop_ends_the_tools_the_turn_started() {
    use std::io::BufRead as _;
    use std::os::unix::process::CommandExt as _;
    let talking = Talking::default();
    let steering = Steering::default();
    let _guard = talking.begin("conv_1", &steering).expect("the turn");
    let mut cli = std::process::Command::new("sh")
        .args(["-c", "sleep 30 & echo $!; wait"])
        .stdout(std::process::Stdio::piped())
        .process_group(0)
        .spawn()
        .expect("sh");
    let mut said = String::new();
    std::io::BufReader::new(cli.stdout.take().expect("stdout"))
        .read_line(&mut said)
        .expect("the tool's pid");
    let tool: i32 = said.trim().parse().expect("a pid");
    talking.on_start("conv_1", false)(cli.id());

    assert!(talking.stop("conv_1", Signal::Kill));
    let cli_ended = ended_within(&mut cli, std::time::Duration::from_secs(5));
    let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while devpit_pty::process::alive(tool) && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let tool_alive = devpit_pty::process::alive(tool);
    let _ = cli.kill();
    let _ = cli.wait();
    unsafe { libc::kill(tool, libc::SIGKILL) };
    assert!(cli_ended, "the CLI outlived the forced stop");
    assert!(!tool_alive, "the tool outlived the forced stop");
}
