use std::sync::{Arc, Mutex};

use devpit_rpc::{Frame, Part};
use tauri::ipc::{Channel, InvokeResponseBody};

use super::{Relay, Sink};

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

/// A channel that keeps every frame it is sent, as sent.
fn heard_whole() -> (Channel<Frame>, Arc<Mutex<Vec<Frame>>>) {
    let got = Arc::new(Mutex::new(Vec::new()));
    let into = got.clone();
    let channel = Channel::new(move |body: InvokeResponseBody| {
        into.lock()
            .unwrap()
            .push(body.deserialize::<Frame>().unwrap());
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
    let (turn, _running) = relay.open("conv", Sink::whole(first));
    turn.send(frame("one")).unwrap();

    let (late, to_late) = heard();
    assert!(relay
        .join("conv", Sink::whole(late.clone()), late.id())
        .is_some());
    turn.send(frame("two")).unwrap();

    assert_eq!(*to_first.lock().unwrap(), ["one", "two"]);
    assert_eq!(*to_late.lock().unwrap(), ["one", "two"]);
}

#[test]
fn nothing_to_join_once_the_turn_is_over() {
    let relay = Relay::default();
    let (first, _) = heard();
    let (_turn, running) = relay.open("conv", Sink::whole(first));
    drop(running);
    let (late, _) = heard();
    assert!(relay
        .join("conv", Sink::whole(late.clone()), late.id())
        .is_none());
}

#[test]
fn a_refused_second_turn_does_not_take_the_running_ones_place() {
    let relay = Relay::default();
    let (first, _) = heard();
    let (turn, _running) = relay.open("conv", Sink::whole(first));
    let (second, _) = heard();
    let (_, refused) = relay.open("conv", Sink::whole(second));
    drop(refused);

    turn.send(frame("still")).unwrap();
    let (late, to_late) = heard();
    assert!(relay
        .join("conv", Sink::whole(late.clone()), late.id())
        .is_some());
    assert_eq!(*to_late.lock().unwrap(), ["still"]);
}

#[test]
fn a_turn_woken_while_the_persons_waits_is_heard_first_and_the_persons_after() {
    let relay = Relay::default();
    let (mine, _) = heard();
    let (person, waiting) = relay.waiting("conv", Sink::whole(mine));
    person.send(frame("asked")).unwrap();
    let (nobody, _) = heard();
    let (woken, woke) = relay.open("conv", Sink::whole(nobody));
    woken.send(frame("woke")).unwrap();

    let (late, to_late) = heard();
    let mut ended = relay
        .join("conv", Sink::whole(late.clone()), late.id())
        .unwrap();
    assert_eq!(*to_late.lock().unwrap(), ["woke"]);
    drop(woke);
    assert!(ended.try_recv().is_ok());

    waiting.begin();
    person.send(frame("answered")).unwrap();
    assert!(relay
        .join("conv", Sink::whole(late.clone()), late.id())
        .is_some());
    assert_eq!(*to_late.lock().unwrap(), ["woke", "asked", "answered"]);
}

#[test]
fn a_chat_that_left_is_sent_nothing_more() {
    let relay = Relay::default();
    let (first, _) = heard();
    let (turn, _running) = relay.open("conv", Sink::whole(first));
    let (gone, to_gone) = heard();
    let mut ended = relay
        .join("conv", Sink::whole(gone.clone()), gone.id())
        .unwrap();
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
    let (turn, _running) = relay.open("conv", Sink::whole(first));
    let (window, to_window) = heard();
    // What the page sends to chat_leave: its own channel's id.
    let page_knows = window.id();
    let id = window.id();
    let mut ended = relay.join("conv", Sink::window(window), id).unwrap();
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

/// A streamed thought is one part, closed with how long it took, even with a
/// subagent's words between its start and its end.
#[test]
fn a_thought_is_kept_once_with_how_long_it_took() {
    let thought = |ms: Option<u32>| devpit_rpc::Part::Thinking {
        text: String::new(),
        parent: None,
        duration_ms: ms,
    };
    let mut parts = Vec::new();
    super::kept(&mut parts, thought(None));
    super::kept(&mut parts, text("inner", Some("toolu_1")));
    super::kept(&mut parts, thought(Some(4200)));
    assert_eq!(
        parts,
        vec![thought(Some(4200)), text("inner", Some("toolu_1"))]
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
    let relay = Relay::default();
    let (turn, _running) = relay.open("conv", Sink::window(window));
    let (remote, whole) = heard_whole();
    let _joined = relay.join("conv", Sink::whole(remote), 0);
    turn.send(frame.clone()).unwrap();
    // The Remote is sent it whole, and cuts its own way.
    let Frame::Part {
        part: Part::ToolResult { output: sent, .. },
        ..
    } = &whole.lock().unwrap()[0]
    else {
        panic!("a tool result");
    };
    assert_eq!(sent.len(), long.len());
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
    let (turn, _running) = relay.open("conv", Sink::window(window));
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

#[test]
fn a_frame_told_to_a_running_turn_reaches_its_window_and_a_late_joiner_in_order() {
    let relay = Relay::default();
    let (first, to_first) = heard();
    let (turn, _running) = relay.open("conv", Sink::whole(first));
    turn.send(frame("asked")).unwrap();
    assert!(relay.told("conv", frame("steered")));
    turn.send(frame("answered")).unwrap();

    let (late, to_late) = heard();
    assert!(relay
        .join("conv", Sink::whole(late.clone()), late.id())
        .is_some());
    assert_eq!(*to_first.lock().unwrap(), ["asked", "steered", "answered"]);
    assert_eq!(*to_late.lock().unwrap(), ["asked", "steered", "answered"]);
}

#[test]
fn nothing_is_told_when_no_turn_runs() {
    let relay = Relay::default();
    assert!(!relay.told("conv", frame("steered")));
}

#[test]
fn a_watcher_hears_each_turn_that_enters_a_conversation_once_it_can_be_joined() {
    let relay = Relay::default();
    let entered = Arc::new(Mutex::new(Vec::new()));
    let into = entered.clone();
    let watch = relay.watch(move |relay, conversation| {
        into.lock()
            .unwrap()
            .push((conversation.to_owned(), relay.running(conversation)));
    });
    let (first, _) = heard();
    let (_turn, running) = relay.open("conv", Sink::whole(first));
    drop(running);
    let (second, _) = heard();
    let (_waits, _waiting) = relay.waiting("other", Sink::whole(second));

    drop(watch);
    let (third, _) = heard();
    let _after = relay.open("conv", Sink::whole(third));
    assert_eq!(
        *entered.lock().unwrap(),
        [("conv".to_owned(), true), ("other".to_owned(), true)]
    );
}

/// Every frame a window is sent while an answer streams fits the payload
/// Tauri evaluates as one script; past it, each frame costs the window a fetch.
#[test]
fn every_frame_a_streaming_answer_sends_a_window_fits_the_budget() {
    use std::time::{Duration, Instant};

    let sizes = Arc::new(Mutex::new(Vec::<usize>::new()));
    let into = sizes.clone();
    let window = Channel::new(move |body: InvokeResponseBody| {
        if let InvokeResponseBody::Json(json) = body {
            into.lock().unwrap().push(json.len());
        }
        Ok(())
    });
    let relay = Relay::default();
    let (turn, _running) = relay.open("conv", Sink::window(window));
    let part = |part: Part| Frame::Part {
        message_id: "m".to_owned(),
        part,
    };
    let start = Instant::now();
    let mut pace = crate::chat_pace::Pace::default();
    let mut sent = |frame: Frame, at: u64| {
        let now = start + Duration::from_millis(at);
        for out in pace.due(now).into_iter().chain(pace.took(frame, now)) {
            turn.send(out).unwrap();
        }
    };
    // Text as fast as the CLI streams it, then a burst of it at once.
    for at in 0..200 {
        sent(part(text("abcdefghijkl", None)), at * 8);
    }
    for at in 0..40 {
        sent(part(text(&"burst of prose ".repeat(70), None)), 1_600 + at);
    }
    // And a whole block in one piece, as a message that was not streamed brings it.
    sent(part(text(&"é whole block ".repeat(1_500), None)), 1_650);
    // A tool's long output, and a screenshot.
    sent(
        part(Part::ToolResult {
            call_id: "c1".to_owned(),
            output: "y".repeat(1_000_000),
            is_error: false,
            parent: None,
        }),
        1_700,
    );
    let image = serde_json::json!([{ "type": "image", "source": { "media_type": "image/png", "data": "A".repeat(2_000_000) } }]);
    sent(
        part(Part::ToolResult {
            call_id: "c2".to_owned(),
            output: image.to_string(),
            is_error: false,
            parent: None,
        }),
        1_710,
    );
    if let Some(frame) = pace.rest() {
        turn.send(frame).unwrap();
    }

    let sizes = sizes.lock().unwrap();
    assert!(
        sizes.len() > 10,
        "frames reached the window: {}",
        sizes.len()
    );
    let over: Vec<_> = sizes
        .iter()
        .filter(|size| **size >= super::FRAME_BUDGET)
        .collect();
    assert!(
        over.is_empty(),
        "frames over {} bytes: {over:?}",
        super::FRAME_BUDGET
    );
}
