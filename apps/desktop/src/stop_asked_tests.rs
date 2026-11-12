use devpit_agentcli::person::{From, Said};

use super::{allowed, ask, swapped, take, waiting};

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

/// The sessions running, as the guard reads a message against them.
const LIVE: [&str; 2] = ["fix-login", "review-x"];

fn names(stops: &[devpit_rpc::StopAsked]) -> Vec<&str> {
    stops.iter().map(|one| one.name.as_str()).collect()
}

#[test]
fn a_stop_nobody_asked_for_is_refused() {
    let name = "fix-login";
    let now = |said: Option<&Said>| allowed(said, name, &LIVE, false, None, false);
    assert!(now(Some(&said("pare fix-login", From::Composer))).is_ok());
    assert!(now(Some(&said("stop review-x fix-login", From::Composer))).is_ok());
    let refused = now(Some(&said("como está a fix-login?", From::Composer)));
    assert!(refused.is_err_and(|why| why.contains("fix-login")));
    assert!(now(None).is_err(), "nothing said");
    assert!(
        now(Some(&said("pare fix-login", From::Channel("telegram")))).is_err(),
        "a channel's yes is its button"
    );
}

#[test]
fn a_session_relieved_on_its_card_is_stopped_without_a_word() {
    let ok = said("ok", From::Composer);
    assert!(allowed(Some(&ok), "fix-login", &LIVE, true, None, false).is_ok());
    assert!(allowed(None, "fix-login", &LIVE, true, None, false).is_ok());
}

#[test]
fn a_stop_the_words_do_not_cover_waits_for_the_persons_click() {
    let words = said("se sim pode encerrar as sessões", From::Composer);
    let why = allowed(Some(&words), "fix-login", &LIVE, false, None, false)
        .expect_err("not on these words");
    let answer = ask("p-click", "fix-login", None, None, &why);
    assert_eq!(answer["done"], false);
    assert!(answer["note"]
        .as_str()
        .is_some_and(|note| note.contains("click")));
    assert_eq!(names(&waiting("p-click")), ["fix-login"]);
    // Asked again, it is still one stop to click.
    ask("p-click", "fix-login", None, None, &why);
    assert_eq!(names(&waiting("p-click")), ["fix-login"]);
}

#[test]
fn one_click_stops_every_session_asked_and_only_those() {
    for name in ["a", "b", "c"] {
        ask("p-batch", name, None, None, "asked");
    }
    let clicked = ["a", "c", "never-asked"].map(str::to_owned);
    assert_eq!(names(&take("p-batch", &clicked)), ["a", "c"]);
    assert_eq!(names(&waiting("p-batch")), ["b"]);
    assert!(waiting("p-other").is_empty(), "another orchestrator's");
}

#[test]
fn what_a_stop_would_lose_goes_inside_its_confirmation() {
    let word = said("stop fix-login", From::Composer);
    let losing = "its folder has 3 file(s) not committed";
    let why = allowed(Some(&word), "fix-login", &LIVE, false, Some(losing), false)
        .expect_err("not without the person seeing it");
    assert!(why.contains(losing));
    assert!(
        allowed(Some(&word), "fix-login", &LIVE, false, Some(losing), true).is_ok(),
        "forced on the person's word"
    );
    ask("p-lose", "fix-login", None, Some(losing.to_owned()), &why);
    assert_eq!(waiting("p-lose")[0].losing.as_deref(), Some(losing));
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
