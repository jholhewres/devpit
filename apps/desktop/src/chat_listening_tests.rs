use std::sync::{mpsc, Arc, Mutex};

use devpit_agentcli::resident::Heard;
use devpit_agentcli::talk::Said;
use devpit_rpc::{Frame, Part, TurnEnd};
use tauri::ipc::{Channel, InvokeResponseBody};

use super::{taken, Ear, Listening, Woken};
use crate::chat_relay::Relay;

fn text(said: &str) -> Part {
    Part::Text {
        text: said.to_owned(),
        parent: None,
    }
}

/// A woken turn, and the frames it sends.
fn woken(relay: &Relay) -> (Woken, Arc<Mutex<Vec<Frame>>>) {
    let got = Arc::new(Mutex::new(Vec::new()));
    let into = got.clone();
    let (frames, relaying) = relay.open(
        "conv",
        Channel::new(move |body: InvokeResponseBody| {
            if let Ok(frame) = body.deserialize::<Frame>() {
                into.lock().unwrap().push(frame);
            }
            Ok(())
        }),
    );
    let turn = Woken {
        turn_id: "turn_woken".to_owned(),
        answer_id: "msg_woken".to_owned(),
        parts: Vec::new(),
        frames,
        _relaying: relaying,
    };
    (turn, got)
}

fn ended() -> Said {
    Said {
        end: TurnEnd {
            turn_id: String::new(),
            cost_usd: Some(0.5),
            duration_ms: None,
            stop_reason: None,
            is_error: false,
            context: None,
        },
        session_id: None,
        init: None,
        anchor: None,
    }
}

fn ear_on(turn: Woken) -> Arc<Ear> {
    let ear = Arc::new(Ear::default());
    ear.now.lock().unwrap().listening = Listening::Woken(turn);
    ear
}

/// Until the claim on its own thread is inside its wait.
fn until_waiting(ear: &Ear) {
    while !ear.now.lock().unwrap().waiting {
        std::thread::yield_now();
    }
}

fn is_woken(ear: &Ear) -> bool {
    matches!(ear.now.lock().unwrap().listening, Listening::Woken(_))
}

#[test]
fn a_person_never_takes_a_process_a_woken_turn_is_on() {
    let relay = Relay::default();
    let (turn, _) = woken(&relay);
    let (tell, _) = mpsc::channel();
    let mut listening = Listening::Woken(turn);
    assert!(!taken(&mut listening, &tell));
    assert!(matches!(listening, Listening::Woken(_)));

    let mut quiet = Listening::Nobody;
    assert!(taken(&mut quiet, &tell));
    assert!(matches!(quiet, Listening::Person(_)));
}

#[test]
fn the_person_waits_for_the_woken_turn_then_hears_only_their_own() {
    let dir = tempfile::tempdir().unwrap();
    let transcript = dir.path().join("conv.jsonl");
    let head = dir.path().join("conv.json");
    let relay = Relay::default();
    let (turn, _) = woken(&relay);
    let ear = ear_on(turn);
    let (tell, heard) = mpsc::channel();
    let claiming = {
        let ear = ear.clone();
        std::thread::spawn(move || ear.claim(&tell))
    };
    until_waiting(&ear);
    assert!(is_woken(&ear), "the woken turn is not taken from under it");

    ear.hear(Heard::Ended(ended()), || unreachable!(), &transcript, &head);
    assert!(claiming.join().unwrap());
    ear.hear(
        Heard::Part(text("mine")),
        || panic!("the person's words started a woken turn"),
        &transcript,
        &head,
    );
    assert!(matches!(heard.try_recv(), Ok(Heard::Part(_))));
}

#[test]
fn stopping_a_waiting_turn_leaves_the_woken_one_running() {
    let relay = Relay::default();
    let (turn, _) = woken(&relay);
    let ear = ear_on(turn);
    assert!(!ear.stop_waiting(), "nothing waits yet");
    let (tell, _) = mpsc::channel();
    let claiming = {
        let ear = ear.clone();
        std::thread::spawn(move || ear.claim(&tell))
    };
    until_waiting(&ear);

    assert!(ear.stop_waiting());
    assert!(!claiming.join().unwrap());
    assert!(is_woken(&ear));
    assert!(!ear.now.lock().unwrap().waiting);
}

#[test]
fn a_woken_turn_that_dies_keeps_what_it_said_and_names_itself() {
    let dir = tempfile::tempdir().unwrap();
    let transcript = dir.path().join("conv.jsonl");
    let relay = Relay::default();
    let (turn, frames) = woken(&relay);
    let ear = ear_on(turn);
    let head = dir.path().join("conv.json");
    ear.hear(
        Heard::Part(text("half an ")),
        || unreachable!(),
        &transcript,
        &head,
    );
    ear.hear(Heard::Gone, || unreachable!(), &transcript, &head);

    let (kept, _) = devpit_agentcli::store::read(&transcript);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].turn_id.as_deref(), Some("turn_woken"));
    assert!(matches!(&kept[0].parts[..], [Part::Text { text, .. }] if text == "half an "));
    let last = frames.lock().unwrap().pop();
    assert!(
        matches!(&last, Some(Frame::Ended { end }) if end.turn_id == "turn_woken"),
        "the end names the turn it ends"
    );
    assert!(matches!(
        ear.now.lock().unwrap().listening,
        Listening::Nobody
    ));
}
