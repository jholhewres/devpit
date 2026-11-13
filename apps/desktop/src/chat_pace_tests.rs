use std::sync::mpsc;
use std::time::{Duration, Instant};

use devpit_rpc::{CallState, Frame, Part};
use tauri::ipc::{Channel, InvokeResponseBody};

use super::{paced, Pace, WINDOW};

fn text(said: &str, parent: Option<&str>) -> Frame {
    Frame::Part {
        message_id: "m1".to_owned(),
        part: Part::Text {
            text: said.to_owned(),
            parent: parent.map(str::to_owned),
        },
    }
}

fn call() -> Frame {
    Frame::Part {
        message_id: "m1".to_owned(),
        part: Part::ToolCall {
            id: "c1".to_owned(),
            name: "Read".to_owned(),
            input: "{}".to_owned(),
            state: CallState::Running,
            parent: None,
        },
    }
}

fn said(frames: &[Frame]) -> Vec<String> {
    frames
        .iter()
        .map(|frame| match frame {
            Frame::Part {
                part: Part::Text { text, .. },
                ..
            } => text.clone(),
            Frame::Part { .. } => "<call>".to_owned(),
            _ => "<other>".to_owned(),
        })
        .collect()
}

#[test]
fn text_written_in_pieces_goes_out_at_most_once_a_window_and_whole() {
    let mut pace = Pace::default();
    let start = Instant::now();
    let mut out = Vec::new();
    // What the CLI does: a dozen characters every 8 ms, for 800 ms.
    for at in 0..100u64 {
        let now = start + Duration::from_millis(at * 8);
        out.extend(pace.due(now));
        out.extend(pace.took(text("abcdefghijkl", None), now));
    }
    out.extend(pace.rest());

    let windows = (800 / WINDOW.as_millis() + 1) as usize;
    assert!(
        out.len() <= windows,
        "{} frames for {windows} windows",
        out.len()
    );
    assert_eq!(said(&out).concat(), "abcdefghijkl".repeat(100));
}

#[test]
fn a_tool_call_lets_out_the_text_before_it_first() {
    let mut pace = Pace::default();
    let now = Instant::now();
    assert!(pace.took(text("Let me ", None), now).is_empty());
    assert!(pace.took(text("look.", None), now).is_empty());
    assert_eq!(said(&pace.took(call(), now)), ["Let me look.", "<call>"]);
    assert!(pace.rest().is_none());
}

#[test]
fn text_of_another_speaker_is_not_joined_onto_it() {
    let mut pace = Pace::default();
    let now = Instant::now();
    assert!(pace.took(text("mine", None), now).is_empty());
    assert_eq!(
        said(&pace.took(text("a subagent's", Some("task_1")), now)),
        ["mine"]
    );
    assert_eq!(
        said(&pace.rest().into_iter().collect::<Vec<_>>()),
        ["a subagent's"]
    );
}

#[test]
fn text_with_nothing_after_it_goes_out_when_its_window_passes() {
    let mut pace = Pace::default();
    let start = Instant::now();
    assert!(pace.took(text("thinking it over", None), start).is_empty());
    assert!(pace
        .due(start + WINDOW - Duration::from_millis(1))
        .is_none());
    let late = pace.due(start + WINDOW).into_iter().collect::<Vec<_>>();
    assert_eq!(said(&late), ["thinking it over"]);
}

fn received() -> (Channel<Frame>, mpsc::Receiver<Frame>) {
    let (tx, rx) = mpsc::channel();
    let channel = Channel::new(move |body: InvokeResponseBody| {
        if let Ok(frame) = body.deserialize::<Frame>() {
            let _ = tx.send(frame);
        }
        Ok(())
    });
    (channel, rx)
}

#[test]
fn a_paced_channel_sends_held_text_on_its_own_and_keeps_the_order() {
    let (window, heard) = received();
    let turn = paced(window);
    turn.send(text("first ", None)).unwrap();
    turn.send(text("words", None)).unwrap();
    // Nothing comes after: the text still arrives, a window later.
    let wait = Duration::from_secs(5);
    assert_eq!(said(&[heard.recv_timeout(wait).unwrap()]), ["first words"]);

    turn.send(text("then", None)).unwrap();
    turn.send(call()).unwrap();
    let next = [
        heard.recv_timeout(wait).unwrap(),
        heard.recv_timeout(wait).unwrap(),
    ];
    assert_eq!(said(&next), ["then", "<call>"]);
}
