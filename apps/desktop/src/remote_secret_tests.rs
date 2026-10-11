use std::path::{Path, PathBuf};

use devpit_core::Store;

use super::{admitted, keep};
use crate::remote_devices::{fresh_token, Devices};

const VALUE: &str = "sk-remote-value-that-is-never-logged";

struct Machine {
    _dir: tempfile::TempDir,
    /// devpit's home: the store, the devices, the Remote's log.
    home: PathBuf,
    store: Store,
    project: String,
    checkout: PathBuf,
}

fn machine() -> Machine {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).expect("home");
    let store = Store::open(&home.join("state.db")).expect("store");
    let checkout = dir.path().join("api");
    std::fs::create_dir_all(&checkout).expect("checkout");
    assert!(std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&checkout)
        .status()
        .expect("git")
        .success());
    let project = store.add_project(&checkout, None).expect("project");
    store.ensure_board(&project).expect("board");
    Machine {
        _dir: dir,
        home,
        store,
        project,
        checkout,
    }
}

fn paired(home: &Path, typing: bool) -> String {
    let mut devices = Devices::read(home);
    let token = fresh_token().expect("token");
    let id = devices.pair("Pixel", &token, 1.0).expect("paired").id;
    devices.allow(&id, typing, false);
    devices.write(home).expect("write");
    id
}

/// Every file devpit keeps under its home, read as bytes.
fn everything_kept(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(everything_kept(&path));
        } else {
            found.push((path.clone(), std::fs::read(&path).expect("read")));
        }
    }
    found
}

fn says(bytes: &[u8], text: &str) -> bool {
    bytes
        .windows(text.len())
        .any(|window| window == text.as_bytes())
}

#[test]
fn a_secret_from_the_remote_lands_in_the_env_and_nothing_devpit_keeps_holds_its_value() {
    let m = machine();
    let device = paired(&m.home, true);
    let kept = keep(
        &m.store,
        &m.home,
        &device,
        &m.project,
        " JEV_API_KEY ",
        VALUE,
        10.0,
    )
    .expect("kept");
    assert_eq!(kept, "JEV_API_KEY");

    let env = m.checkout.join(".env");
    assert_eq!(
        std::fs::read_to_string(&env).expect("env"),
        format!("JEV_API_KEY={VALUE}\n")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&env).expect("meta").permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    let log = std::fs::read_to_string(m.home.join("remote-log.jsonl")).expect("log");
    assert!(log.contains("kept the secret JEV_API_KEY"), "{log}");
    for (path, bytes) in everything_kept(&m.home) {
        assert!(!says(&bytes, VALUE), "{} holds the value", path.display());
    }

    let refused = keep(
        &m.store, &m.home, &device, &m.project, "bad name", VALUE, 11.0,
    )
    .expect_err("a name is capitals");
    assert!(!refused.contains(VALUE), "a refusal never repeats it");
}

#[test]
fn an_oversize_value_from_the_remote_is_refused_before_anything_is_written() {
    let m = machine();
    let device = paired(&m.home, true);
    let huge = "x".repeat(crate::vault::LONGEST_VALUE + 1);
    let why =
        keep(&m.store, &m.home, &device, &m.project, "BIG", &huge, 10.0).expect_err("too big");
    assert!(why.contains("8 KB"), "{why}");
    assert!(!m.checkout.join(".env").exists());
}

#[test]
fn only_a_paired_device_allowed_to_type_keeps_a_secret() {
    let m = machine();
    let keeping = |device: &str| keep(&m.store, &m.home, device, &m.project, "KEY", VALUE, 10.0);

    let never = keeping("dev_never_paired").expect_err("not paired");
    assert!(never.contains("not paired"), "{never}");

    let watcher = paired(&m.home, false);
    assert!(keeping(&watcher)
        .expect_err("watches")
        .contains("only watch"));

    // Forgotten after its connection was let in: the pairing is read again.
    let phone = paired(&m.home, true);
    let mut devices = Devices::read(&m.home);
    devices.forget(&phone);
    devices.write(&m.home).expect("write");
    assert!(keeping(&phone).is_err());

    assert!(!m.checkout.join(".env").exists());
}

#[test]
fn a_device_keeps_only_a_few_secrets_a_minute() {
    let mut times = Vec::new();
    for at in 0..5 {
        assert!(admitted(&mut times, f64::from(at)));
    }
    assert!(!admitted(&mut times, 10.0), "the sixth in a minute");
    assert!(admitted(&mut times, 61.0), "a minute after the first");

    let m = machine();
    let device = paired(&m.home, true);
    for at in 0..5 {
        keep(
            &m.store,
            &m.home,
            &device,
            &m.project,
            "KEY",
            "v",
            100.0 + f64::from(at),
        )
        .expect("within the limit");
    }
    let why = keep(&m.store, &m.home, &device, &m.project, "KEY", "v", 106.0)
        .expect_err("past the limit");
    assert!(why.contains("a minute"), "{why}");
}
