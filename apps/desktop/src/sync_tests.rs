use std::collections::BTreeMap;
use std::sync::Mutex;

use devpit_sync::{Blobs, Listed};

use super::*;

/// devpit.app's sync, in memory: every blob as it was sent.
#[derive(Default)]
struct Kept(Mutex<BTreeMap<String, (u64, Vec<u8>)>>);

impl Blobs for Kept {
    async fn list(&self) -> Result<Vec<Listed>, SyncError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .iter()
            .map(|(name, (version, _))| Listed {
                name: name.clone(),
                version: *version,
            })
            .collect())
    }

    async fn get(&self, name: &str) -> Result<Option<(u64, Vec<u8>)>, SyncError> {
        Ok(self.0.lock().unwrap().get(name).cloned())
    }

    async fn put(&self, name: &str, base: u64, bytes: Vec<u8>) -> Result<u64, SyncError> {
        let mut kept = self.0.lock().unwrap();
        if kept.get(name).map(|(version, _)| *version).unwrap_or(0) != base {
            return Err(SyncError::Stale);
        }
        kept.insert(name.to_owned(), (base + 1, bytes));
        Ok(base + 1)
    }

    async fn clear(&self) -> Result<(), SyncError> {
        self.0.lock().unwrap().clear();
        Ok(())
    }
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

#[test]
fn sync_is_off_until_it_is_turned_on() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    assert!(!on(&store));
    store
        .set_preference_flag(preference::SYNC, true)
        .expect("on");
    assert!(on(&store));
}

#[cfg(unix)]
#[tokio::test]
async fn a_secret_kept_in_a_projects_vault_never_syncs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("thing");
    std::fs::create_dir_all(&root).expect("dir");
    assert!(std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .status()
        .expect("git")
        .success());
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let project = store
        .add_project(&root, Some("git@github.com:jo/thing.git"))
        .expect("add");
    store
        .edit_project(&project, "Thing", None, Some("rocket"), None)
        .expect("edit");
    // Through the vault's own write, as Edit project → Secrets keeps one.
    let names = dir.path().join("home").join("secrets.json");
    crate::vault::keep(&root, &names, "STRIPE_KEY", "sk_live_VAULTSENTINEL0123").expect("keep");
    assert!(std::fs::read_to_string(root.join(".env"))
        .expect("the .env")
        .contains("VAULTSENTINEL"));

    let key = SyncKey::generate().expect("key");
    let kept = Kept::default();
    let mut state = devpit_sync::State::new();
    let machine = state.machine.clone();
    let (prefs, projects, machines) = namespaces(&store, &machine, "laptop", now_ms());
    let spaces: [&dyn Namespace; 3] = [&prefs, &projects, &machines];
    devpit_sync::round(&kept, &key, &mut state, &spaces, now_ms())
        .await
        .expect("round");

    let blobs = kept.0.lock().unwrap().clone();
    assert_eq!(blobs.len(), 3);
    let mut words = String::new();
    for (name, (version, bytes)) in &blobs {
        for secret in ["VAULTSENTINEL", "STRIPE_KEY", "sk_live"] {
            assert!(!contains(bytes, secret), "{secret} crossed in a blob");
        }
        let opened = devpit_sync::seal::open(&key, name, *version, bytes).expect("opens");
        words.push_str(&String::from_utf8(opened).expect("utf-8"));
    }
    assert!(words.contains("rocket"), "the project's settings synced");
    for secret in [
        "VAULTSENTINEL",
        "STRIPE_KEY",
        "sk_live",
        ".env",
        "secrets.json",
    ] {
        assert!(!words.contains(secret), "{secret} was synced");
    }
}
