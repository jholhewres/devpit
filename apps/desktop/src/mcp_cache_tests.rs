use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use super::{Kept, Known, FRESH_FOR};

/// Long enough for a spawned ask to have run, were one spawned.
const AN_ASK_WOULD_HAVE_RUN: Duration = Duration::from_millis(200);

/// Waits for `key` to read `wanted`, without a fixed sleep.
fn until(kept: &Kept<u32>, key: &str, wanted: Known<u32>) {
    let began = Instant::now();
    while kept.known(key, FRESH_FOR) != wanted {
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "never became {wanted:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_fresh_answer_is_read_and_nothing_is_asked() {
    let kept = Kept::default();
    kept.put("claude|/p", 3);
    let (asked, heard) = mpsc::channel();
    let known = kept.read_and_refresh("claude|/p", FRESH_FOR, move || {
        asked.send(()).ok();
        Some(4)
    });
    assert_eq!(known, Known::Fresh(3));
    assert!(
        heard.recv_timeout(AN_ASK_WOULD_HAVE_RUN).is_err(),
        "asked again"
    );
}

#[test]
fn a_start_never_waits_on_the_ask() {
    let kept = Arc::new(Kept::default());
    let (go, wait) = mpsc::channel::<()>();
    // The ask blocks until the test lets it go: the read returning first is
    // the read not waiting. Read on a thread, so a read that waits fails
    // here rather than hanging the suite.
    let (read, answered) = mpsc::channel();
    let reader = Arc::clone(&kept);
    std::thread::spawn(move || {
        read.send(reader.read_and_refresh("k", FRESH_FOR, move || {
            wait.recv().ok();
            Some(7)
        }))
    });
    let known = answered
        .recv_timeout(Duration::from_secs(2))
        .expect("the start waited on the ask");
    assert_eq!(known, Known::Unknown);
    // A second start while the first ask runs neither waits nor asks again.
    let (asked, heard) = mpsc::channel();
    let again = kept.read_and_refresh("k", FRESH_FOR, move || {
        asked.send(()).ok();
        Some(8)
    });
    assert_eq!(again, Known::Unknown);
    go.send(()).expect("let it answer");
    until(&kept, "k", Known::Fresh(7));
    assert!(
        heard.recv_timeout(AN_ASK_WOULD_HAVE_RUN).is_err(),
        "asked twice"
    );
}

#[test]
fn a_stale_answer_is_read_while_a_new_one_is_asked_behind_it() {
    let kept = Kept::default();
    kept.put("k", 1);
    // Nothing is fresh for no time at all.
    let known = kept.read_and_refresh("k", Duration::ZERO, || Some(2));
    assert_eq!(known, Known::Stale(1));
    until(&kept, "k", Known::Fresh(2));
}

#[test]
fn a_failed_ask_keeps_the_old_answer_and_can_be_asked_again() {
    let kept = Kept::default();
    kept.put("k", 1);
    let (done, finished) = mpsc::channel();
    kept.read_and_refresh("k", Duration::ZERO, move || {
        done.send(()).ok();
        None
    });
    finished.recv().expect("asked");
    assert_eq!(kept.known("k", Duration::ZERO), Known::Stale(1));
    // Asked again once the failed one has let go of the key.
    let began = Instant::now();
    while kept.known("k", FRESH_FOR) != Known::Fresh(5) {
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "the key stayed taken"
        );
        kept.read_and_refresh("k", Duration::ZERO, || Some(5));
        std::thread::sleep(Duration::from_millis(5));
    }
}
