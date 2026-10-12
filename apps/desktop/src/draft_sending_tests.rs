use devpit_agentcli::person::{From, Said};

use super::{key_of, lets_go, spend};

fn said(id: &str, text: &str) -> Said {
    Said {
        id: id.to_owned(),
        text: text.to_owned(),
        from: From::RemoteControl,
    }
}

#[test]
fn a_message_naming_two_sessions_sends_to_each_once() {
    let mut held = Vec::new();
    let both = said("u1", "manda a noiseless e a ascbot seguirem");
    assert!(spend(&mut held, key_of("p", &both, "noiseless-mvp")));
    assert!(spend(&mut held, key_of("p", &both, "ascbot-lotus")));
    assert!(!spend(&mut held, key_of("p", &both, "noiseless-mvp")));
}

#[test]
fn a_bare_send_is_spent_whole_whichever_draft_it_sent() {
    let mut held = Vec::new();
    let go = said("u1", "envie");
    assert!(spend(&mut held, key_of("p", &go, "noiseless-mvp")));
    // A draft written after it is not sent on the same old word.
    assert!(!spend(&mut held, key_of("p", &go, "ascbot-lotus")));
    // The next thing the person says is a new word.
    assert!(spend(
        &mut held,
        key_of("p", &said("u2", "envie"), "ascbot-lotus")
    ));
}

#[test]
fn a_typed_yes_from_a_channel_lets_no_draft_go() {
    let on_telegram = |text: &str| Said {
        id: "u1".to_owned(),
        text: text.to_owned(),
        from: From::Channel("telegram"),
    };
    // There the yes is a press on devpit's button, never the words.
    assert!(!lets_go(&on_telegram("envie"), "noiseless-mvp", 1));
    assert!(!lets_go(
        &on_telegram("manda a noiseless-mvp seguir"),
        "noiseless-mvp",
        1
    ));
    // The same words from anywhere else do.
    assert!(lets_go(&said("u2", "envie"), "noiseless-mvp", 1));
}
