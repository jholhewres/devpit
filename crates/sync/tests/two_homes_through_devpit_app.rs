//! Two devpit homes of one account syncing through devpit-app itself, on
//! loopback: its API over a Postgres, its relay carrying the key from one
//! machine to the other, and every blob it keeps read back raw.
//!
//! It needs a devpit-app checkout and a database it may write to, so it runs
//! only when asked:
//!
//! ```sh
//! DEVPIT_APP_DIR=../devpit-app DATABASE_URL=postgres://…/devpit_app_test \
//!   cargo test -p devpit-sync --test two_homes_through_devpit_app
//! ```

mod support;

use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use devpit_core::preference as key;
use devpit_remote::Relay;
use devpit_sync::pairing::{self, Offer};
use devpit_sync::seal::{open, Unsealed};
use devpit_sync::{files, Blobs, DevpitApp, SyncKey, MOST_BLOB};
use serde_json::{json, Value};
use support::{holds, Home};

/// devpit-app, stopped with the test.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port()
}

async fn started(app: &str, database: &str) -> (Running, String) {
    let port = free_port();
    let child = Command::new("cargo")
        .args(["run", "-q", "-p", "devpit-app-api"])
        .env("DATABASE_URL", database)
        .env("DEVPIT_APP_ADDR", format!("127.0.0.1:{port}"))
        .env("RUST_LOG", "warn")
        .current_dir(app)
        .stdout(Stdio::null())
        .spawn()
        .expect("cargo run devpit-app");
    let running = Running(child);
    let origin = format!("http://127.0.0.1:{port}");
    let until = Instant::now() + Duration::from_secs(600);
    while Instant::now() < until {
        if reqwest::get(format!("{origin}/healthz"))
            .await
            .is_ok_and(|answer| answer.status().is_success())
        {
            return (running, origin);
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    panic!("devpit-app did not start");
}

/// A session of the account `email`, as the cookie's value a bearer carries.
async fn session(origin: &str, path: &str, email: &str) -> String {
    let answer = reqwest::Client::new()
        .post(format!("{origin}{path}"))
        .json(&json!({ "email": email, "password": "a-long-enough-password-for-sync" }))
        .send()
        .await
        .expect("sign in");
    assert!(answer.status().is_success(), "{path}: {}", answer.status());
    answer
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|cookie| cookie.strip_prefix("dp_session="))
        .and_then(|rest| rest.split(';').next())
        .expect("a session cookie")
        .to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_preference_crosses_two_homes_through_devpit_app_which_reads_none_of_it() {
    let (Ok(app), Ok(database)) = (
        std::env::var("DEVPIT_APP_DIR"),
        std::env::var("DATABASE_URL"),
    ) else {
        eprintln!("skipped: DEVPIT_APP_DIR and DATABASE_URL name no devpit-app and no database");
        return;
    };
    let (_app, origin) = started(&app, &database).await;
    let email = format!("{}@example.com", ulid::Ulid::generate()).to_lowercase();
    let token_a = session(&origin, "/v1/auth/register", &email).await;
    let token_b = session(&origin, "/v1/auth/login", &email).await;

    // Machine A makes the key and syncs what it chose.
    let (mut a, mut b) = (Home::new("laptop"), Home::new("desktop"));
    let made = SyncKey::generate().expect("key");
    files::keep_key(&a.root(), &made).expect("keep");
    a.store.set_preference(key::THEME, "light").expect("set");
    a.store.set_preference(key::REMOTE, "true").expect("set");
    a.store
        .set_preference(
            key::CALENDAR_LINK,
            "https://devpit.app/cal/CALSECRET987654321.ics",
        )
        .expect("set");
    let project = a
        .store
        .add_project(&a.root().join("thing"), Some("git@github.com:jo/thing.git"))
        .expect("add");
    a.store
        .edit_project(&project, "Thing Of Jo", None, Some("rocket"), None)
        .expect("edit");
    let on_a = DevpitApp::new(&origin, &token_a).expect("client");
    let sent = a.sync(&on_a).await.expect("a syncs");
    assert_eq!(sent.sent.len(), 3, "{sent:?}");

    // What devpit-app keeps, read back raw: none of it says what went in.
    let listed = on_a.list().await.expect("list");
    assert_eq!(listed.len(), 3);
    let stranger = SyncKey::generate().expect("another key");
    for blob in &listed {
        let (version, bytes) = on_a.get(&blob.name).await.expect("get").expect("there");
        for word in [
            "light",
            "appearance.theme",
            "Thing Of Jo",
            "rocket",
            "laptop",
            "prefs",
        ] {
            assert!(!holds(&bytes, word), "{word:?} is readable on devpit-app");
        }
        assert_eq!(
            open(&stranger, &blob.name, version, &bytes),
            Err(Unsealed::OtherKey)
        );
        assert!(
            open(&made, &blob.name, version, &bytes).is_ok(),
            "the key opens it"
        );
    }

    // The key goes to machine B through devpit-app's relay, end to end.
    let (offer, keys) = Offer::new("laptop").expect("offer");
    let relay = Relay::new(&origin, &token_a, &offer.host);
    let (secret, handed) = (offer.secret, made.clone());
    let stop = Arc::new(AtomicBool::new(false));
    let offering = std::thread::spawn(move || {
        pairing::offer(
            &relay,
            &keys,
            &secret,
            &handed,
            Instant::now() + Duration::from_secs(60),
            &stop,
        )
    });
    let carried = Offer::parse(&offer.text()).expect("the text");
    let mut taken = None;
    for _ in 0..50 {
        // The offering machine may not be online on the relay yet.
        match pairing::join(&origin, &token_b, &carried, "desktop") {
            Ok(key) => {
                taken = Some(key);
                break;
            }
            Err(_) => std::thread::sleep(Duration::from_millis(200)),
        }
    }
    let taken = taken.expect("machine B took the key through the relay");
    assert_eq!(taken.to_hex(), made.to_hex());
    assert_eq!(offering.join().expect("thread"), Ok("desktop".to_owned()));
    files::keep_key(&b.root(), &taken).expect("keep");

    // Machine B, on its own session, gets the theme and the project's mark.
    let on_b = DevpitApp::new(&origin, &token_b).expect("client");
    let theirs = b
        .store
        .add_project(&b.root().join("thing"), Some("https://github.com/jo/thing"))
        .expect("add");
    b.sync(&on_b).await.expect("b syncs");
    assert_eq!(b.pref(key::THEME).as_deref(), Some("light"));
    assert_eq!(b.pref(key::REMOTE), None, "machine-specific");
    assert_eq!(b.pref(key::CALENDAR_LINK), None, "machine-specific");
    let row = b.store.project(&theirs).expect("read").expect("there");
    assert_eq!(
        (row.name.as_str(), row.icon.as_deref()),
        ("Thing Of Jo", Some("rocket"))
    );

    // And back: B's change reaches A.
    b.store
        .set_preference(key::TERMINAL_CONTRAST, "11")
        .expect("set");
    b.sync(&on_b).await.expect("b again");
    a.sync(&on_a).await.expect("a again");
    assert_eq!(a.pref(key::TERMINAL_CONTRAST).as_deref(), Some("11"));

    // devpit-app keeps its ceilings: a blob past 1 MiB, a stale version. A
    // client each, since a refusal before the body is read ends the connection.
    let raw = reqwest::Client::new;
    let name = &listed[0].name;
    let past = raw()
        .put(format!("{origin}/v1/sync/{name}?version=0"))
        .bearer_auth(&token_a)
        .body(vec![0u8; MOST_BLOB + 1])
        .send()
        .await
        .expect("put");
    assert_eq!(past.status().as_u16(), 413);
    let stale = raw()
        .put(format!("{origin}/v1/sync/{name}?version=0"))
        .bearer_auth(&token_a)
        .body(vec![1u8; 16])
        .send()
        .await
        .expect("put");
    assert_eq!(stale.status().as_u16(), 409);

    // Another account sees none of it.
    let other = session(
        &origin,
        "/v1/auth/register",
        &format!("{}@example.com", ulid::Ulid::generate()).to_lowercase(),
    )
    .await;
    let theirs: Value = raw()
        .get(format!("{origin}/v1/sync"))
        .bearer_auth(&other)
        .send()
        .await
        .expect("list")
        .json()
        .await
        .expect("json");
    assert_eq!(theirs["blobs"], json!([]));
    let missing = raw()
        .get(format!("{origin}/v1/sync/{name}"))
        .bearer_auth(&other)
        .send()
        .await
        .expect("get");
    assert_eq!(missing.status().as_u16(), 404);
}
