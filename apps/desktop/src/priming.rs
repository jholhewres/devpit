//! Making a fresh checkout usable, as a setting.
//!
//! Apart from `worktrees.rs` because it is a different question: that file
//! owns which checkouts exist and what removing one costs, and this one owns
//! what is done to a new one before anybody works in it.

use devpit_rpc::{ErrorCode, RpcError};

use crate::prime::{self, Prime};
use crate::worktrees::{home, Preparation};

/// `worktree.prime.read` — what this project does to a fresh checkout.
#[tauri::command]
#[specta::specta]
pub fn worktree_prime_read(project_id: String) -> Result<Preparation, RpcError> {
    Ok(prime::read(&prime::prime_path(&home()?, &project_id)).into())
}

/// `worktree.prime.write` — save it, refusing a command that is not installed.
///
/// Refused here rather than when a card lands on a column: a typo should be
/// answered while the person is still looking at what they typed.
#[tauri::command]
#[specta::specta]
pub fn worktree_prime_write(
    project_id: String,
    preparation: Preparation,
) -> Result<Preparation, RpcError> {
    let declared: Prime = preparation.into();
    let missing = prime::missing(&declared);
    if !missing.is_empty() {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            format!("not installed: {}", missing.join(", ")),
        ));
    }
    prime::write(&prime::prime_path(&home()?, &project_id), &declared)
        .map_err(|err| RpcError::new(ErrorCode::Internal, err.to_string()))?;
    Ok(declared.into())
}
