use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use ed25519_dalek::{Signer as _, SigningKey};

use super::{status_with, verified};

/// The same token devpit.app issues: `base64url(payload).base64url(signature)`.
fn issued(key: &SigningKey, expires: i64) -> String {
    let payload =
        format!(r#"{{"v":1,"account":"acc_1","plan":"yearly","expires":{expires},"issued":0}}"#);
    let signature = key.sign(payload.as_bytes());
    format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(&payload),
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    )
}

fn key(seed: u8) -> (SigningKey, String) {
    let key = SigningKey::from_bytes(&[seed; 32]);
    let public = STANDARD.encode(key.verifying_key().to_bytes());
    (key, public)
}

#[test]
fn an_entitlement_from_devpit_app_holds_until_it_expires() {
    let (signer, public) = key(7);
    let token = issued(&signer, 100);
    assert_eq!(
        verified(&public, &token, 99).map(|one| one.plan),
        Some("yearly".to_owned())
    );
    assert!(verified(&public, &token, 100).is_none());
}

#[test]
fn a_forged_or_foreign_entitlement_is_not_plus() {
    let (signer, public) = key(7);
    let (_, other) = key(8);
    let token = issued(&signer, 100);
    assert!(verified(&other, &token, 0).is_none());
    let (payload, signature) = token.split_once('.').expect("two parts");
    let longer = URL_SAFE_NO_PAD.encode(
        String::from_utf8(URL_SAFE_NO_PAD.decode(payload).expect("payload"))
            .expect("text")
            .replace("100", "999999"),
    );
    assert!(verified(&public, &format!("{longer}.{signature}"), 0).is_none());
}

#[test]
fn without_a_key_built_in_plus_is_neither_checked_nor_drawn() {
    let (signer, public) = key(7);
    let token = issued(&signer, 100);
    let off = status_with(None, Some(&token), 0);
    assert!(!off.enabled && !off.active);
    let on = status_with(Some(&public), Some(&token), 0);
    assert!(on.enabled && on.active);
    // Expired: only what needs devpit.app stops, and that is what `active` says.
    assert!(!status_with(Some(&public), Some(&token), 200).active);
    assert!(!status_with(Some(&public), None, 0).active);
}
