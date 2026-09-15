use std::path::PathBuf;

use devpit_agentcli::head::{head_path, read_head};

use super::*;
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
