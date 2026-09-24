use std::sync::{Arc, Mutex};

use devpit_rpc::Frame;
use tauri::ipc::{Channel, InvokeResponseBody};

use super::Relay;

fn heard() -> (Channel<Frame>, Arc<Mutex<Vec<String>>>) {
    let got = Arc::new(Mutex::new(Vec::new()));
    let into = got.clone();
    let channel = Channel::new(move |body: InvokeResponseBody| {
        if let Ok(Frame::Session { session_id }) = body.deserialize::<Frame>() {
            into.lock().unwrap().push(session_id);
        }
        Ok(())
    });
    (channel, got)
}

fn frame(name: &str) -> Frame {
    Frame::Session {
        session_id: name.to_owned(),
    }
}

#[test]
fn a_late_joiner_hears_what_came_before_and_what_comes_after() {
    let relay = Relay::default();
    let (first, to_first) = heard();
    let (turn, _running) = relay.open("conv", first);
    turn.send(frame("one")).unwrap();

    let (late, to_late) = heard();
    assert!(relay.join("conv", late).is_some());
    turn.send(frame("two")).unwrap();

    assert_eq!(*to_first.lock().unwrap(), ["one", "two"]);
    assert_eq!(*to_late.lock().unwrap(), ["one", "two"]);
}

#[test]
fn nothing_to_join_once_the_turn_is_over() {
    let relay = Relay::default();
    let (first, _) = heard();
    let (_turn, running) = relay.open("conv", first);
    drop(running);
    let (late, _) = heard();
    assert!(relay.join("conv", late).is_none());
}

#[test]
fn a_refused_second_turn_does_not_take_the_running_ones_place() {
    let relay = Relay::default();
    let (first, _) = heard();
    let (turn, _running) = relay.open("conv", first);
    let (second, _) = heard();
    let (_, refused) = relay.open("conv", second);
    drop(refused);

    turn.send(frame("still")).unwrap();
    let (late, to_late) = heard();
    assert!(relay.join("conv", late).is_some());
    assert_eq!(*to_late.lock().unwrap(), ["still"]);
}
