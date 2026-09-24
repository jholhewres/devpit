//! A panic lands in the error file, once the person has switched reports on.
//!
//! Its own binary because the panic hook and the switch are process-wide.

use devpit_core::reports::{keep_panics, switch, ErrorLog};

#[test]
fn a_panic_is_kept_when_asked() {
    let dir = tempfile::tempdir().expect("tempdir");
    keep_panics();

    let _ = std::thread::spawn(|| panic!("nobody was supposed to get here")).join();
    assert!(
        !ErrorLog::at(dir.path()).path().exists(),
        "off keeps nothing"
    );

    switch(dir.path(), true).expect("on");
    let _ = std::thread::spawn(|| panic!("nobody was supposed to get here")).join();

    let kept = ErrorLog::at(dir.path()).read();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].kind, "panic");
    assert_eq!(kept[0].message, "nobody was supposed to get here");
    assert!(
        kept[0]
            .location
            .as_deref()
            .is_some_and(|at| at.contains("a_panic_is_kept_when_asked.rs")),
        "{:?}",
        kept[0].location
    );
    assert!(kept[0].stack.is_some());
}
