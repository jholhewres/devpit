use devpit_agentcli::person::{From, Said};

use super::{allowed, swapped};

fn said(text: &str, from: From) -> Said {
    Said {
        id: "u1".into(),
        text: text.into(),
        from,
    }
}

fn live(name: &str, pid: i32, card: Option<&str>) -> devpit_rpc::LiveSession {
    devpit_rpc::LiveSession {
        name: name.to_owned(),
        pid,
        job: None,
        status: "busy".to_owned(),
        kind: "interactive".to_owned(),
        cwd: "/w".to_owned(),
        project_id: Some("prj_a".to_owned()),
        project_name: None,
        card_id: card.map(str::to_owned),
        since: None,
        in_devpit: true,
        waiting: None,
        pane: None,
        session_id: None,
        step: None,
        draft: None,
        remote: Default::default(),
    }
}

#[test]
fn a_stop_nobody_asked_for_is_refused() {
    let name = "fix-login";
    assert!(allowed(Some(&said("pare a fix-login", From::Composer)), name, false).is_ok());
    let refused = allowed(
        Some(&said("como está a fix-login?", From::Composer)),
        name,
        false,
    );
    assert!(refused.is_err_and(|why| why.contains("fix-login")));
    assert!(allowed(None, name, false).is_err(), "nothing said");
    assert!(
        allowed(
            Some(&said("pare a fix-login", From::Channel("telegram"))),
            name,
            false
        )
        .is_err(),
        "a channel's yes is its button"
    );
}

#[test]
fn a_session_relieved_on_its_card_is_stopped_without_a_word() {
    assert!(allowed(Some(&said("ok", From::Composer)), "fix-login", true).is_ok());
    assert!(allowed(None, "fix-login", true).is_ok());
}

#[test]
fn a_swap_is_another_session_running_on_the_same_card() {
    let old = live("fix-login", 10, Some("card_a"));
    let relief = live("fix-login-2", 11, Some("card_a"));
    let elsewhere = live("other", 12, Some("card_b"));
    let free = live("free", 13, None);
    assert!(swapped(&[old.clone(), relief.clone()], "fix-login", None));
    assert!(
        !swapped(&[old.clone(), elsewhere.clone()], "fix-login", None),
        "no relief"
    );
    assert!(
        !swapped(&[free.clone(), live("free", 14, None)], "free", None),
        "no card"
    );
    let same_name = live("fix-login", 11, Some("card_a"));
    assert!(
        swapped(&[old.clone(), same_name.clone()], "fix-login", Some(10)),
        "the old one by pid"
    );
    assert!(
        !swapped(&[old, same_name], "fix-login", None),
        "both would stop"
    );
    assert!(!swapped(&[], "fix-login", None), "nothing running");
}
