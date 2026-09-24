//! An `Internal` made anywhere lands in the error file with the place that
//! made it, and nothing else does.
//!
//! Its own binary because the switch is process-wide.

use devpit_core::reports::{switch, ErrorLog};
use devpit_core::StoreError;
use devpit_rpc::{ErrorCode, RpcError};

fn through_the_store() -> Result<(), RpcError> {
    Err(StoreError::NoDataDirectory)?
}

#[test]
fn an_internal_error_is_kept_with_its_place() {
    let dir = tempfile::tempdir().expect("tempdir");
    switch(dir.path(), true).expect("on");

    let _ = RpcError::internal("the thing broke");
    let _ = through_the_store();
    let _ = RpcError::new(ErrorCode::NotFound, "no such card");
    let _ = RpcError::forbidden("outside the project root");

    let kept = ErrorLog::at(dir.path()).read();
    let messages: Vec<_> = kept.iter().map(|entry| entry.message.as_str()).collect();
    assert_eq!(kept.len(), 2, "only Internal is a bug: {messages:?}");
    for entry in &kept {
        let at = entry.location.as_deref().unwrap_or_default();
        assert!(
            at.contains("an_internal_error_is_kept_with_its_place.rs"),
            "the place is the caller's, not error.rs: {at}"
        );
    }
}
