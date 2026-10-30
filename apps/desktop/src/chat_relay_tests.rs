use std::sync::{Arc, Mutex};

use devpit_rpc::{Frame, Part};
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
    assert!(relay.join("conv", late.clone(), late.id()).is_some());
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
    assert!(relay.join("conv", late.clone(), late.id()).is_none());
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
    assert!(relay.join("conv", late.clone(), late.id()).is_some());
    assert_eq!(*to_late.lock().unwrap(), ["still"]);
}

#[test]
fn a_turn_woken_while_the_persons_waits_is_heard_first_and_the_persons_after() {
    let relay = Relay::default();
    let (mine, _) = heard();
    let (person, waiting) = relay.waiting("conv", mine);
    person.send(frame("asked")).unwrap();
    let (nobody, _) = heard();
    let (woken, woke) = relay.open("conv", nobody);
    woken.send(frame("woke")).unwrap();

    let (late, to_late) = heard();
    let mut ended = relay.join("conv", late.clone(), late.id()).unwrap();
    assert_eq!(*to_late.lock().unwrap(), ["woke"]);
    drop(woke);
    assert!(ended.try_recv().is_ok());

    waiting.begin();
    person.send(frame("answered")).unwrap();
    assert!(relay.join("conv", late.clone(), late.id()).is_some());
    assert_eq!(*to_late.lock().unwrap(), ["woke", "asked", "answered"]);
}

#[test]
fn a_chat_that_left_is_sent_nothing_more() {
    let relay = Relay::default();
    let (first, _) = heard();
    let (turn, _running) = relay.open("conv", first);
    let (gone, to_gone) = heard();
    let mut ended = relay.join("conv", gone.clone(), gone.id()).unwrap();
    relay.leave("conv", gone.id());
    turn.send(frame("after")).unwrap();

    assert!(to_gone.lock().unwrap().is_empty());
    // Left, not ended: the rejoin stops instead of waiting for the next turn.
    assert!(matches!(
        ended.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Closed)
    ));
}

/// The window rejoins through the cutting wrapper and leaves by its own
/// channel's id, which is the one the page knows.
#[test]
fn a_chat_that_rejoined_cut_and_left_is_sent_nothing_more() {
    let relay = Relay::default();
    let (first, _) = heard();
    let (turn, _running) = relay.open("conv", first);
    let (window, to_window) = heard();
    // What the page sends to chat_leave: its own channel's id.
    let page_knows = window.id();
    let (cut, id) = super::window_channel(window);
    let mut ended = relay.join("conv", cut, id).unwrap();
    turn.send(frame("before")).unwrap();
    relay.leave("conv", page_knows);
    turn.send(frame("after")).unwrap();

    assert_eq!(to_window.lock().unwrap().len(), 1);
    assert!(matches!(
        ended.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Closed)
    ));
}

fn text(said: &str, parent: Option<&str>) -> devpit_rpc::Part {
    devpit_rpc::Part::Text {
        text: said.to_owned(),
        parent: parent.map(str::to_owned),
    }
}

/// A streamed answer is kept as one part, and a subagent's words stay apart.
#[test]
fn streamed_text_is_kept_whole_per_speaker() {
    let mut parts = Vec::new();
    for piece in ["FIRST ", "DONE ", "banana"] {
        super::kept(&mut parts, text(piece, None));
    }
    super::kept(&mut parts, text("inner", Some("toolu_1")));
    assert_eq!(
        parts,
        vec![
            text("FIRST DONE banana", None),
            text("inner", Some("toolu_1"))
        ]
    );
}

/// A late joiner is handed the pieces so far as one frame, not hundreds.
#[test]
fn a_streamed_answer_is_relayed_whole_to_a_late_joiner() {
    let mut frames = Vec::new();
    for piece in ["a", "b", "c"] {
        let part = text(piece, None);
        super::kept_frame(
            &mut frames,
            Frame::Part {
                message_id: "m".to_owned(),
                part,
            },
        );
    }
    assert_eq!(frames.len(), 1);
    let Frame::Part { part, .. } = &frames[0] else {
        panic!("not a part")
    };
    assert_eq!(part, &text("abc", None));
}

#[test]
fn a_window_is_sent_a_long_output_cut_and_the_relay_keeps_it_whole() {
    let heard = Arc::new(Mutex::new(Vec::<Frame>::new()));
    let into = heard.clone();
    let window = Channel::new(move |body: InvokeResponseBody| {
        into.lock()
            .unwrap()
            .push(body.deserialize::<Frame>().unwrap());
        Ok(())
    });
    let long = "y".repeat(50_000);
    let frame = Frame::Part {
        message_id: "m".to_owned(),
        part: Part::ToolResult {
            call_id: "c".to_owned(),
            output: long.clone(),
            is_error: false,
            parent: None,
        },
    };
    super::for_window(window).send(frame.clone()).unwrap();
    let Frame::Part {
        part: Part::ToolResult { output, .. },
        ..
    } = &heard.lock().unwrap()[0]
    else {
        panic!("a tool result");
    };
    assert!(output.len() < 5_000 && output.contains(devpit_rpc::cut::CUT_MARK));
    let Frame::Part {
        part: Part::ToolResult { output: kept, .. },
        ..
    } = frame
    else {
        unreachable!()
    };
    assert_eq!(kept, long);
}

/// A turn still running is not in the transcript yet: its whole outputs are
/// read from what the relay kept.
#[test]
fn a_running_turns_output_is_found_whole_in_the_relay() {
    let relay = Relay::default();
    let (window, _) = heard();
    let (turn, _running) = relay.open("conv", super::for_window(window));
    let long = "y".repeat(50_000);
    turn.send(Frame::Part {
        message_id: "m".to_owned(),
        part: Part::ToolResult {
            call_id: "c".to_owned(),
            output: long.clone(),
            is_error: false,
            parent: None,
        },
    })
    .unwrap();
    let Some(Part::ToolResult { output, .. }) = relay.running_output("conv", "m", "c") else {
        panic!("the output");
    };
    assert_eq!(output, long);
    assert_eq!(relay.running_output("conv", "m", "other"), None);
    assert_eq!(relay.running_output("elsewhere", "m", "c"), None);
}
