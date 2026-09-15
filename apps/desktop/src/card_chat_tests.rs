use std::path::{Path, PathBuf};

use devpit_agentcli::head::{head_path, read_head};

use super::*;
use crate::adopting::adopt;
use crate::chat_turn::turn_cwd;

struct Seeded {
    dir: tempfile::TempDir,
    store: Store,
    project: String,
    card: String,
    root: PathBuf,
    checkout: PathBuf,
}

fn seeded() -> Seeded {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let root = dir.path().join("project");
    let checkout = dir.path().join("checkout");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::create_dir_all(&checkout).expect("checkout");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "Wire the board", "")
        .expect("card");
    store
        .set_card_front(&card, checkout.to_str(), Some("main"))
        .expect("checkout");
    Seeded {
        dir,
        store,
        project,
        card,
        root,
        checkout,
    }
}

#[test]
fn a_card_chat_keeps_its_checkout_after_the_card_is_deleted() {
    let seeded = seeded();
    let sessions = seeded.dir.path().join("sessions");
    open_card_chat(
        &seeded.store,
        &sessions,
        &seeded.project,
        &seeded.card,
        "conv_1",
        &seeded.checkout,
        None,
    )
    .expect("opened");
    assert_eq!(
        seeded.store.chat_card("conv_1").expect("read").as_deref(),
        Some(seeded.card.as_str())
    );

    seeded.store.delete_card(&seeded.card).expect("delete");
    assert_eq!(seeded.store.chat_card("conv_1").expect("read"), None);
    let head = read_head(&head_path(&sessions, "conv_1")).expect("head");
    assert_eq!(head.title.as_deref(), Some("Wire the board"));
    // The next turn still runs in the checkout, whatever the window asks for.
    assert_eq!(
        turn_cwd(head.cwd.as_deref(), "/somewhere/else").expect("runs"),
        seeded.checkout.display().to_string()
    );
}

#[test]
fn a_card_chat_is_fixed_only_to_its_checkout_or_the_project() {
    let seeded = seeded();
    let sessions = seeded.dir.path().join("sessions");
    open_card_chat(
        &seeded.store,
        &sessions,
        &seeded.project,
        &seeded.card,
        "conv_root",
        &seeded.root,
        Some("prof_1".to_owned()),
    )
    .expect("the project root");

    let refused = open_card_chat(
        &seeded.store,
        &sessions,
        &seeded.project,
        &seeded.card,
        "conv_elsewhere",
        seeded.dir.path(),
        None,
    )
    .expect_err("elsewhere");
    assert_eq!(refused.code, ErrorCode::Invalid);
    assert!(read_head(&head_path(&sessions, "conv_elsewhere")).is_none());
    assert_eq!(
        seeded.store.chat_card("conv_elsewhere").expect("read"),
        None
    );
}

fn agent_run(seeded: &Seeded, card: &str, session: &str, cwd: Option<&Path>) {
    let step = seeded
        .store
        .create_step(&seeded.project, "agent", "review", "{}", false)
        .expect("step");
    let run = seeded.store.start_run(card, &step, None).expect("run");
    seeded
        .store
        .set_run_session(&run, session)
        .expect("session");
    if let Some(cwd) = cwd {
        seeded
            .store
            .set_run_cwd(&run, &cwd.display().to_string())
            .expect("cwd");
    }
}

fn card_without_checkout(seeded: &Seeded) -> String {
    let column = seeded.store.columns(&seeded.project).expect("columns")[0]
        .id
        .clone();
    seeded
        .store
        .create_card(&seeded.project, &column, "Review it", "")
        .expect("card")
}

#[test]
fn an_adopted_run_resumes_where_it_ran() {
    let seeded = seeded();
    // An agent step runs at the project root: it asks for no checkout.
    let card = card_without_checkout(&seeded);
    agent_run(&seeded, &card, "s-run", Some(&seeded.root));

    let cwd =
        card_session_cwd(&seeded.store, &seeded.project, &card, "s-run").expect("where it ran");
    assert_eq!(cwd, seeded.root.display().to_string());

    let sessions = seeded.dir.path().join("sessions");
    adopt(
        &sessions,
        "conv_run",
        "s-run",
        "prof_1",
        None,
        Some(cwd.clone()),
        1.0,
    )
    .expect("adopted");
    seeded.store.link_chat(&card, "conv_run").expect("filed");
    let head = read_head(&head_path(&sessions, "conv_run")).expect("head");
    assert_eq!(head.cwd.as_deref(), Some(cwd.as_str()));
    assert_eq!(head.session_id.as_deref(), Some("s-run"));
    // And no checkout was made to take it in.
    assert_eq!(
        seeded
            .store
            .card(&card)
            .expect("read")
            .expect("card")
            .worktree_path,
        None
    );
    let named = conversation_card(&seeded.store, "conv_run").expect("card");
    assert_eq!(named.title.as_deref(), Some("Review it"));
    assert!(named.on_board);
}

#[test]
fn a_session_whose_folder_was_not_recorded_is_refused() {
    let seeded = seeded();
    let card = card_without_checkout(&seeded);
    agent_run(&seeded, &card, "s-run", None);
    let refused =
        card_session_cwd(&seeded.store, &seeded.project, &card, "s-run").expect_err("no cwd");
    assert_eq!(refused.code, ErrorCode::Invalid);
    assert!(
        refused.message.contains("not recorded"),
        "{}",
        refused.message
    );

    seeded
        .store
        .link_session(&card, "a1b2", "s-bg", None, None)
        .expect("link");
    assert!(card_session_cwd(&seeded.store, &seeded.project, &card, "s-bg").is_err());

    // A run recorded somewhere that is neither the checkout nor the project.
    agent_run(&seeded, &card, "s-away", Some(seeded.dir.path()));
    let away = card_session_cwd(&seeded.store, &seeded.project, &card, "s-away").expect_err("away");
    assert_eq!(away.code, ErrorCode::Invalid);
    // A card is only reached through its own project, even from a real one.
    let elsewhere = seeded.dir.path().join("other");
    std::fs::create_dir_all(&elsewhere).expect("other root");
    let other = seeded
        .store
        .add_project(&elsewhere, None)
        .expect("other project");
    let stranger =
        card_session_cwd(&seeded.store, &other, &card, "s-away").expect_err("other project");
    assert_eq!(stranger.code, ErrorCode::NotFound);

    // A session from the card's terminal needs the card's checkout, never a new one.
    assert!(card_session_cwd(&seeded.store, &seeded.project, &card, "s-pane").is_err());
    assert_eq!(
        card_session_cwd(&seeded.store, &seeded.project, &seeded.card, "s-pane").expect("checkout"),
        seeded.checkout.display().to_string()
    );
}
