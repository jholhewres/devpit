//! What belongs to one machine never leaves it: its paths, windows, tray,
//! remote access and calendar address, and the secrets a project keeps in
//! its vault. Set on one machine with words nobody would write twice, synced
//! to another, and looked for in both the other machine and every blob.

mod support;

use devpit_core::preference as key;
use devpit_sync::{files, SyncKey};
use support::{holds, opened, Home, Stand};

const LOCAL: [(&str, &str); 9] = [
    (key::REMOTE, "true"),
    (key::REMOTE_VIA, "relay"),
    (
        key::CALENDAR_LINK,
        "https://devpit.app/cal/CALENDARSECRET0123456789.ics",
    ),
    (
        key::AGENT_PROFILES,
        r#"[{"id":"work","env":{"ANTHROPIC_AUTH_TOKEN":"sk-profile-token-0123"}}]"#,
    ),
    (key::WORKTREE_BASE, "/home/jo/only-on-this-machine"),
    (key::SIDEBAR_WIDTH, "333"),
    (key::ISLAND_SCREEN, "HDMI-ONLY-HERE"),
    (key::KEEP_IN_TRAY, "false"),
    (key::TRANSCRIBE_MODEL, "/home/jo/models/ggml-only-here.bin"),
];

#[tokio::test]
async fn machine_specific_preferences_do_not_sync() {
    let stand = Stand::default();
    let (mut a, mut b) = (Home::new("laptop"), Home::new("desktop"));
    let made = SyncKey::generate().expect("key");
    files::keep_key(&a.root(), &made).expect("keep");
    files::keep_key(&b.root(), &made).expect("keep");
    for (name, value) in LOCAL {
        a.store.set_preference(name, value).expect("set");
    }
    a.store.set_preference(key::THEME, "light").expect("set");
    a.sync(&stand).await.expect("a");
    b.sync(&stand).await.expect("b");

    assert_eq!(
        b.pref(key::THEME).as_deref(),
        Some("light"),
        "what syncs did"
    );
    let words = opened(&made, &stand.kept());
    for (name, value) in LOCAL {
        assert_eq!(b.pref(name), None, "{name} reached the other machine");
        assert!(!words.contains(name), "{name} was in a blob");
        // A flag's `true` could be anyone's; the words that are this one's
        // alone are looked for.
        if value.len() > 6 {
            assert!(!words.contains(value), "{name}'s value was in a blob");
        }
    }
}

#[tokio::test]
async fn a_projects_vault_never_syncs() {
    let stand = Stand::default();
    let mut a = Home::new("laptop");
    let made = SyncKey::generate().expect("key");
    files::keep_key(&a.root(), &made).expect("keep");
    let root = a.root().join("thing");
    std::fs::create_dir_all(&root).expect("dir");
    // What the vault writes: the value in the project's .env, the name in
    // secrets.json beside it.
    std::fs::write(root.join(".env"), "STRIPE_KEY=sk_live_VAULTSENTINEL0123\n").expect(".env");
    std::fs::write(root.join("secrets.json"), r#"[{"name":"STRIPE_KEY"}]"#).expect("names");
    let project = a
        .store
        .add_project(&root, Some("git@github.com:jo/thing.git"))
        .expect("add");
    a.store
        .edit_project(&project, "Thing", None, Some("rocket"), None)
        .expect("edit");
    a.sync(&stand).await.expect("sync");

    let words = opened(&made, &stand.kept());
    assert!(words.contains("rocket"), "the project's settings synced");
    for secret in ["VAULTSENTINEL", "STRIPE_KEY", "sk_live"] {
        assert!(!words.contains(secret), "{secret} reached a blob");
        for (_, (_, bytes)) in stand.kept() {
            assert!(!holds(&bytes, secret));
        }
    }
    assert!(
        !words.contains(&root.display().to_string()),
        "a path never syncs"
    );
}
