//! The read path's rules, tested by calling them.

use super::*;

fn errno(code: i32) -> std::io::Error {
    std::io::Error::from_raw_os_error(code)
}

/// The errors that are not an ending. Reading them as one closes a
/// terminal that is still working, and the person sees a pane that shut
/// itself for no reason they can name.
#[test]
fn a_signal_or_an_empty_pty_is_not_the_end() {
    for kind in [
        std::io::ErrorKind::Interrupted,
        std::io::ErrorKind::WouldBlock,
        std::io::ErrorKind::TimedOut,
    ] {
        let error = std::io::Error::new(kind, "not an ending");
        assert_eq!(after_read(&error, false), AfterRead::Again, "{kind:?}");
    }
}

/// The race this exists for: a pty master answers `EIO` in the gap between
/// the fork and the child attaching its slave. The child is fine.
#[test]
fn eio_before_the_child_attaches_is_not_the_end() {
    assert_eq!(after_read(&errno(EIO), false), AfterRead::Again);
}

/// The same errno, once the child really has gone.
#[test]
fn eio_after_the_child_exits_is_the_end() {
    assert_eq!(after_read(&errno(EIO), true), AfterRead::Ended);
}

/// Anything else ends the session and says which error it was, rather
/// than ending silently the way every error used to.
#[test]
fn an_error_that_is_none_of_those_is_broken() {
    // EBADF: the descriptor is gone, and no amount of waiting fixes it.
    assert_eq!(after_read(&errno(9), false), AfterRead::Broken);
    assert_eq!(after_read(&errno(9), true), AfterRead::Broken);
}
