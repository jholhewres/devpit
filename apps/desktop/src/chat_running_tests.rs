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
    talking.on_start("conv_1")(child.id());
    let ended = ended_within(&mut child, std::time::Duration::from_secs(5));
    let _ = child.kill();
    let _ = child.wait();
    assert!(ended, "the process outlived the stop asked before it");
}

/// What "Force stop" is for: a turn whose process does not end on the polite
/// signal still ends.
#[cfg(unix)]
#[test]
fn a_forced_stop_ends_a_process_the_polite_one_did_not() {
    let talking = Talking::default();
    let steering = Steering::default();
    let _guard = talking.begin("conv_1", &steering).expect("the turn");
    let mut child = std::process::Command::new("sh")
        .args(["-c", "trap '' TERM; exec sleep 30"])
        .spawn()
        .expect("sh");
    talking.on_start("conv_1")(child.id());
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
