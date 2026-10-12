//! A blob as devpit.app keeps it: XChaCha20-Poly1305 under a key drawn from
//! the sync key, with a nonce drawn at random for every seal.
//!
//! ```text
//! "dps1" | key id (8) | nonce (24) | ciphertext and tag
//! ```
//!
//! What the server could change without the key is bound in as associated
//! data: the header, the blob's name and the version the server is to keep
//! it as. A blob served under another name, or as another version than the
//! one it was sealed for, does not open.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};

use crate::key::SyncKey;

const MAGIC: &[u8; 4] = b"dps1";
const ID: usize = 8;
const NONCE: usize = 24;
const HEADER: usize = MAGIC.len() + ID + NONCE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unsealed {
    /// Not a blob devpit sealed.
    NotSealed,
    /// Sealed with another key: this machine holds a different one.
    OtherKey,
    /// The right key, and it does not open: changed, or served as another
    /// name or version than it was sealed for.
    Tampered,
}

impl std::fmt::Display for Unsealed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Unsealed::NotSealed => "devpit.app holds something that is not a sealed blob",
            Unsealed::OtherKey => {
                "the account's synced data is sealed with another key than this machine's"
            }
            Unsealed::Tampered => {
                "a synced blob did not open: it was changed, or served as another than it was"
            }
        })
    }
}

fn bound(header: &[u8], name: &str, version: u64) -> Vec<u8> {
    let mut aad = Vec::with_capacity(header.len() + name.len() + 12);
    aad.extend_from_slice(header);
    aad.extend_from_slice(&(name.len() as u32).to_be_bytes());
    aad.extend_from_slice(name.as_bytes());
    aad.extend_from_slice(&version.to_be_bytes());
    aad
}

/// `plain`, sealed to be kept as `name` at `version`.
pub fn seal(key: &SyncKey, name: &str, version: u64, plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce = [0u8; NONCE];
    getrandom::fill(&mut nonce).map_err(|_| "the system gave no randomness for a nonce")?;
    let mut header = Vec::with_capacity(HEADER);
    header.extend_from_slice(MAGIC);
    header.extend_from_slice(&key.id_bytes());
    let aad = bound(&header, name, version);
    header.extend_from_slice(&nonce);
    let cipher = XChaCha20Poly1305::new(&key.sealing().into());
    let sealed = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plain,
                aad: &aad,
            },
        )
        .map_err(|_| "a blob could not be sealed")?;
    header.extend_from_slice(&sealed);
    Ok(header)
}

/// The id of the key `sealed` was sealed with, when it is a sealed blob.
pub fn sealed_by(sealed: &[u8]) -> Option<[u8; ID]> {
    if sealed.len() < HEADER || &sealed[..MAGIC.len()] != MAGIC {
        return None;
    }
    let mut id = [0u8; ID];
    id.copy_from_slice(&sealed[MAGIC.len()..MAGIC.len() + ID]);
    Some(id)
}

/// What `sealed` holds, when it was sealed with `key` as `name` at `version`.
pub fn open(key: &SyncKey, name: &str, version: u64, sealed: &[u8]) -> Result<Vec<u8>, Unsealed> {
    let id = sealed_by(sealed).ok_or(Unsealed::NotSealed)?;
    if id != key.id_bytes() {
        return Err(Unsealed::OtherKey);
    }
    let (header, body) = sealed.split_at(HEADER);
    let aad = bound(&header[..MAGIC.len() + ID], name, version);
    let nonce = XNonce::from_slice(&header[MAGIC.len() + ID..]);
    XChaCha20Poly1305::new(&key.sealing().into())
        .decrypt(
            nonce,
            Payload {
                msg: body,
                aad: &aad,
            },
        )
        .map_err(|_| Unsealed::Tampered)
}

#[cfg(test)]
#[path = "seal_tests.rs"]
mod tests;
