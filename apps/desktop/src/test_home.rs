//! The tests' own home. `Store::root` is under HOME, and a bare `cargo test
//! -p devpit-desktop` wrote a project folder and a store row into the dev
//! build's `~/.devpit-dev` for every test that registers one. Set before the
//! first test runs, as `make test-rust`'s `TEST_HOME` is.

use std::path::{Path, PathBuf};

/// `<target>/test-home`, beside the test binary in `<target>/<profile>/deps`.
pub(crate) fn home() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    exe.ancestors().nth(3).map_or_else(
        || std::env::temp_dir().join("devpit-test-home"),
        |target| target.join("test-home"),
    )
}

#[ctor::ctor]
fn isolate() {
    let home = home();
    let at = |dir: &str| home.join(dir);
    for dir in [".cache", ".config", ".local/share"] {
        let _ = std::fs::create_dir_all(at(dir));
    }
    // A shell started by a test reads these, and finds nothing of the person's.
    for file in [".zshenv", ".zshrc", ".bashrc", ".profile"] {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(at(file));
    }
    let keep = |key: &str, dir: &str| {
        if std::env::var_os(key).is_none() {
            if let Some(was) = std::env::var_os("HOME") {
                set(key, &Path::new(&was).join(dir));
            }
        }
    };
    keep("CARGO_HOME", ".cargo");
    keep("RUSTUP_HOME", ".rustup");
    set("HOME", &home);
    set("XDG_CACHE_HOME", &at(".cache"));
    set("XDG_CONFIG_HOME", &at(".config"));
    set("XDG_DATA_HOME", &at(".local/share"));
    // Windows finds the profile folder without HOME: devpit's root is named outright.
    set("DEVPIT_HOME", &at(devpit_core::ROOT_NAME));
}

/// Before main, so before any test thread reads the environment.
fn set(key: &str, value: &Path) {
    std::env::set_var(key, value);
}

#[cfg(test)]
#[path = "test_home_tests.rs"]
mod tests;
