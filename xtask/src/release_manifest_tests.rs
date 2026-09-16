//! What the release publishes, and what it refuses to publish.
//!
//! The fixture is real: a throwaway key pair signed the payload below, and the
//! second signature comes from a different key. Only public halves are here.

use super::*;

/// The payload both signatures were made over.
const PAYLOAD: &[u8] = b"hello from the release\n";

/// The throwaway public key, base64 as it sits in `tauri.conf.json`.
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDcxMDE5QTFEQ0UyRTkxQTkKUldTcGtTN09IWm9CY1hJZEh0Tkg3MERPM0hVR1UyaVR6V3g2VE9CaU5mUk9HR1RGb0UyNGRzaFEK";

/// Its signature over PAYLOAD, base64 as it is published.
const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTcGtTN09IWm9CY1dHYytZcDZZSVJ1RzNVZjVOOTZqcTZWR0ZMZ0VCTHVibmQyWWdtU0hnV3U0Q2dJUXR1Z2FyOThvcUJQM1VLREE2Zkt3SHBPNVNPSzNSSCt1SUhNbEEwPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzg5NTI4ODQ1CWZpbGU6YXJ0aWZhY3QuYmluCmFlU1RMN29GQXMrOUh1VFI3Y1p6RHoxZW40Rmd6SkMzRGNpSTVGYW1SYStHMUVteXVncS9pZnY1MVJ4ZVVDR1I4My95RkNYdWtpRkxFS3BkWURIRUNRPT0K";

/// A signature over the same bytes, made by another key.
const ANOTHER_KEYS_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVSVzZLTFJybGF3dGFaeVBLM2E2SFRwZzVyckpWckl4TnFKajVZS1RldlBzUy85ZjJoSmptY0ZGV292d3hGL2VnV1hHdlA2dEc4dlBvTTE3MWdWMnk2VEl6U05ZR1dJR2djPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzg5NTI4ODcwCWZpbGU6b3RoZXIuYmluCnUvd3huYzVVaXM0UkVLUXB5UWRremZJaXFJL3ZSR0psMDRaZFZWVDREb2UySTRUeUtRUkR0UEFxWW5hdXZKK0xVc0dmdGNQS3A0bVJtaEdodjN1ZkF3PT0K";

#[test]
fn a_signature_from_the_key_the_app_carries_is_accepted() {
    assert_eq!(verified(PUBKEY, SIGNATURE, PAYLOAD), Ok(()));
}

/// The sabotage this command exists for: an artifact signed with a different
/// key is refused before anything is written.
#[test]
fn a_signature_from_another_key_is_refused() {
    let refused = verified(PUBKEY, ANOTHER_KEYS_SIGNATURE, PAYLOAD)
        .expect_err("another key's signature was accepted");
    assert!(refused.contains("different key"), "{refused}");

    // And so is a signature over other bytes.
    assert!(verified(PUBKEY, SIGNATURE, b"not what was signed").is_err());
}

#[test]
fn the_manifest_is_the_shape_the_updater_reads() {
    let written = manifest(
        "0.2.0",
        "what changed",
        "2026-09-16T00:00:00Z",
        "linux-x86_64",
        "https://example.invalid/devpit_0.2.0_amd64.AppImage",
        SIGNATURE,
    );

    assert_eq!(written["version"], "0.2.0");
    assert_eq!(written["notes"], "what changed");
    assert_eq!(written["pub_date"], "2026-09-16T00:00:00Z");
    let platform = &written["platforms"]["linux-x86_64"];
    assert_eq!(
        platform["url"],
        "https://example.invalid/devpit_0.2.0_amd64.AppImage"
    );
    assert_eq!(platform["signature"], SIGNATURE);
}

#[test]
fn the_checksums_are_the_format_sha256sum_reads() {
    let said = checksums(&[("devpit.deb".to_owned(), b"one".to_vec())]);
    // sha256 of "one", then two spaces, then the name.
    assert_eq!(
        said,
        "7692c3ad3540bb803c020b3aee66cd8887123234ea0c6e7143c0add73ff431ed  devpit.deb\n"
    );
}

#[test]
fn the_app_carries_a_public_key_to_verify_against() {
    assert!(pubkey_of(&crate::workspace_root()).is_some());
}
