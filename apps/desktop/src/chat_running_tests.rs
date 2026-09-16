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
