//! The ownership rule, tested by calling it.

use super::*;
use portable_pty::{ChildKiller, MasterPty};
use std::io::Write;

/// The race this exists for: a React remount attaches before the old attach
/// has noticed it is over. The newer one wins, and the older one's detach
/// arriving afterwards must not take the pane away from it.
#[test]
fn the_later_attach_wins_and_the_earlier_detach_does_nothing() {
    let claims = Claims::new();
    let first = claims.take_for("leaf", "a").expect("first claim");
    let second = claims.take_for("leaf", "b").expect("second claim");
    assert!(second.generation > first.generation);

    assert!(
        !claims.release("leaf", "a").expect("release"),
        "a replaced client released a claim it no longer held"
    );
    assert!(claims.release("leaf", "b").expect("release"));
}

/// The regression that made every terminal in development print an error
/// instead of a shell: the screen used one client id for the whole window, so
/// a remount reattached under the id that already held the pane. Nothing
/// about reusing an id may refuse the newer attach.
#[test]
fn the_same_client_can_attach_twice() {
    let claims = Claims::new();
    let first = claims.take_for("leaf", "same").expect("first claim");
    let second = claims.take_for("leaf", "same").expect("second claim");

    assert!(second.generation > first.generation);
    assert!(
        !claims
            .install("leaf", first.generation, live("same"))
            .expect("stale install"),
        "the attach that was replaced installed its pty anyway"
    );
    assert!(claims
        .install("leaf", second.generation, live("same"))
        .expect("install"));
}

/// A reload mints ids the app has never seen and cannot order. Whatever they
/// are, the attach that arrives later is the one that owns the pane.
#[test]
fn an_unorderable_id_still_claims_the_pane() {
    let claims = Claims::new();
    let old = claims.take_for("leaf", "zzzz-high").expect("first claim");
    let new = claims.take_for("leaf", "aaaa-low").expect("second claim");

    assert!(new.generation > old.generation);
    assert!(claims
        .install("leaf", new.generation, live("aaaa-low"))
        .expect("install"));
    assert!(claims.live("leaf").is_ok());
}

/// Claiming a pane hands back whoever was live on it, so the caller can stop
/// a pty that nothing is reading any more.
#[test]
fn claiming_hands_back_the_client_it_replaced() {
    let claims = Claims::new();
    let first = claims.take_for("leaf", "a").expect("claim");
    claims
        .install("leaf", first.generation, live("a"))
        .expect("install");

    let second = claims.take_for("leaf", "b").expect("newer claim");
    let replaced = second.replaced.expect("the previous client came back");
    assert_eq!(replaced.client_id, "a");
}

/// Two panes are two claims. A handover on one must not disturb the other.
#[test]
fn panes_are_claimed_independently() {
    let claims = Claims::new();
    let one = claims.take_for("one", "a").expect("claim one");
    claims.take_for("two", "b").expect("claim two");
    claims.take_for("two", "c").expect("newer claim on two");

    assert!(
        claims
            .install("one", one.generation, live("a"))
            .expect("install"),
        "a handover on another pane invalidated this one"
    );
}

/// A pane nobody has attached is a sentence, not a panic.
#[test]
fn a_pane_with_no_client_says_so() {
    let claims = Claims::new();
    let missing = claims.live("never-attached");
    assert!(matches!(
        missing,
        Err(RpcError {
            code: ErrorCode::NotFound,
            ..
        })
    ));
}

/// A `Live` with ends that go nowhere. The ownership rule never touches them;
/// it only ever compares the id and the generation beside them.
fn live(client_id: &str) -> Arc<Live> {
    Arc::new(Live {
        client_id: client_id.to_owned(),
        writer: Mutex::new(Box::new(std::io::sink())),
        master: Mutex::new(Box::new(Nowhere)),
        killer: Mutex::new(Box::new(Nowhere)),
        ring: Arc::new(Mutex::new(devpit_pty::RingBuffer::new(
            devpit_pty::RING_BYTES,
        ))),
        pid: None,
    })
}

#[derive(Debug)]
struct Nowhere;

impl MasterPty for Nowhere {
    fn resize(&self, _size: portable_pty::PtySize) -> anyhow::Result<()> {
        Ok(())
    }
    fn get_size(&self) -> anyhow::Result<portable_pty::PtySize> {
        Ok(portable_pty::PtySize::default())
    }
    fn try_clone_reader(&self) -> anyhow::Result<Box<dyn std::io::Read + Send>> {
        Ok(Box::new(std::io::empty()))
    }
    fn take_writer(&self) -> anyhow::Result<Box<dyn Write + Send>> {
        Ok(Box::new(std::io::sink()))
    }
    #[cfg(unix)]
    fn process_group_leader(&self) -> Option<i32> {
        None
    }
    #[cfg(unix)]
    fn as_raw_fd(&self) -> Option<std::os::fd::RawFd> {
        None
    }
    #[cfg(unix)]
    fn tty_name(&self) -> Option<std::path::PathBuf> {
        None
    }
}

impl ChildKiller for Nowhere {
    fn kill(&mut self) -> std::io::Result<()> {
        Ok(())
    }
    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(Nowhere)
    }
}
