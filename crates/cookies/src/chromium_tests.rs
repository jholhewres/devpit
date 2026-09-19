//! The shape of the file and the shape of the key.

use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};

use super::*;

type Encryptor = cbc::Encryptor<aes::Aes128>;

/// Encrypts the way Chromium does, so the reader can be tested against
/// something it did not also produce the expectations for.
fn sealed(plain: &[u8], key: &[u8; 16], version: &[u8]) -> Vec<u8> {
    let mut buffer = vec![0_u8; plain.len() + 16];
    buffer[..plain.len()].copy_from_slice(plain);
    let body = Encryptor::new(key.into(), &IV.into())
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, plain.len())
        .expect("room for the padding");
    let mut out = version.to_vec();
    out.extend_from_slice(body);
    out
}

/// The fallback password is not a secret and never was — it is what Chromium
/// uses when no keyring answered. Pinned so a change to the salt or the
/// iteration count cannot pass unnoticed: both produce a key that decrypts
/// nothing, and that failure looks exactly like a wrong password.
#[test]
fn the_fallback_key_is_the_one_chromium_derives() {
    let key = key_from(&Password::Fallback);
    assert_eq!(key.len(), 16);
    /* The same password twice gives the same key, and a different one does
    not — which is the whole contract a caller depends on. */
    assert_eq!(key, key_from(&Password::Fallback));
    assert_ne!(
        key,
        key_from(&Password::Keyring("something else".to_owned()))
    );
}

/// Each password has its own prefix, and each opens only its own values.
///
/// My first version of this encrypted BOTH prefixes with the fallback key and
/// called it coverage — which codified the bug under a name that read like a
/// test. On Linux the three bytes name the *password*, not the cipher.
#[test]
fn each_password_opens_the_values_it_wrote_and_says_so_about_the_others() {
    let keyring = Password::Keyring("a secret the keyring holds".to_owned());
    assert_eq!(Password::Fallback.prefix(), *b"v10");
    assert_eq!(keyring.prefix(), *b"v11");

    for password in [Password::Fallback, keyring] {
        let key = key_from(&password);
        let stored = sealed(b"a-session-value", &key, &password.prefix());
        let out = plain(&stored, &key, ".example.com", &password).expect("it decrypts");
        assert_eq!(out.seen(), "a-session-value");
    }
}

/// And a store holding both — a machine where a keyring appeared, or went
/// away — says which it is rather than reporting a corrupt profile.
#[test]
fn a_value_written_under_the_other_password_is_named_and_not_blamed() {
    let keyring = Password::Keyring("a secret".to_owned());
    let written_with_keyring = sealed(b"a-session-value", &key_from(&keyring), b"v11");

    let refused = plain(
        &written_with_keyring,
        &key_from(&Password::Fallback),
        ".example.com",
        &Password::Fallback,
    )
    .expect_err("the fallback cannot open a keyring value");
    assert!(
        matches!(refused, Refused::OtherPassword),
        "got {refused:?}, which reads as a corrupt store"
    );
    let said = refused.to_string();
    assert!(said.contains("keyring"), "{said}");
    /* And a way out. On a machine with no secret-tool the cause alone leaves
    somebody knowing there are two keys and nothing to do about it. */
    assert!(said.contains("libsecret-tools"), "{said}");
}

/// A profile encrypted under another key must refuse rather than return
/// nonsense that a webview would then store as somebody's session.
#[test]
fn a_value_from_another_profile_is_refused_and_not_guessed() {
    let mine = key_from(&Password::Fallback);
    let theirs = key_from(&Password::Keyring("another machine".to_owned()));
    let stored = sealed(b"a-session-value", &theirs, b"v10");
    assert!(matches!(
        plain(&stored, &mine, ".example.com", &Password::Fallback),
        Err(Refused::WrongKey)
    ));
}

/// An old profile, or a machine with encryption off, stores the value as it is.
#[test]
fn a_value_that_was_never_encrypted_is_read_as_it_stands() {
    let key = key_from(&Password::Fallback);
    assert_eq!(
        plain(b"plain-value", &key, ".example.com", &Password::Fallback)
            .unwrap()
            .seen(),
        "plain-value"
    );
    assert!(plain(b"", &key, ".example.com", &Password::Fallback)
        .unwrap()
        .is_empty());
}

/// Chromium 130 puts the SHA-256 of the cookie's own host in front of the
/// plaintext, and comparing that hash is what replaced a heuristic that asked
/// whether the leading bytes looked printable.
#[test]
fn the_real_domain_hash_is_dropped_and_nothing_else_is() {
    use sha2::{Digest, Sha256};
    let key = key_from(&Password::Fallback);
    let host = ".example.com";

    let mut with_hash = Sha256::digest(host.as_bytes()).to_vec();
    with_hash.extend_from_slice(b"the-real-value");
    let stored = sealed(&with_hash, &key, b"v10");
    assert_eq!(
        plain(&stored, &key, host, &Password::Fallback)
            .unwrap()
            .seen(),
        "the-real-value"
    );

    /* Another host's hash is not this cookie's, and stays.

    Asserted on the content and not on a length: `from_utf8_lossy` turns each
    invalid byte of a raw digest into U+FFFD, so the byte count afterwards is
    not the byte count before, and my first version of this compared the two. */
    let mut wrong = Sha256::digest(b"other.test").to_vec();
    wrong.extend_from_slice(b"tail");
    let stored = sealed(&wrong, &key, b"v10");
    let kept = plain(&stored, &key, host, &Password::Fallback).unwrap();
    assert!(
        kept.seen().ends_with("tail"),
        "the tail was lost with the leading bytes"
    );
    assert!(
        kept.seen().len() > 4,
        "32 bytes that are not this host's hash were eaten"
    );
}

/// The defect the heuristic had, proved in review by running it: a value with
/// any byte outside printable ASCII lost its first 32 bytes, silently.
#[test]
fn an_accented_value_keeps_all_of_itself() {
    let key = key_from(&Password::Fallback);
    let value = r#"{"user":"José Pérez","plan":"pro","id":"a1b2c3d4e5f6a7b8c9d0"}"#;
    let stored = sealed(value.as_bytes(), &key, b"v10");
    assert_eq!(
        plain(&stored, &key, ".example.com", &Password::Fallback)
            .unwrap()
            .seen(),
        value,
        "the printable-bytes heuristic is back"
    );
}

/// The other direction, at the boundary: an empty value under Chromium 130
/// decrypts to exactly 32 bytes, and the old length check returned early and
/// handed the whole hash back as the cookie.
#[test]
fn an_empty_value_behind_a_hash_comes_back_empty() {
    use sha2::{Digest, Sha256};
    let key = key_from(&Password::Fallback);
    let host = "example.com";
    let only_hash = Sha256::digest(host.as_bytes()).to_vec();
    let stored = sealed(&only_hash, &key, b"v10");
    assert_eq!(
        plain(&stored, &key, host, &Password::Fallback)
            .unwrap()
            .seen(),
        "",
        "the hash came back as the value"
    );
}

/// Microseconds since 1601, which is nobody else's epoch. Read as seconds
/// since 1970 it lands in the wrong millennium and the webview drops every
/// cookie — an import that looks like it did nothing.
#[test]
fn an_expiry_is_moved_off_the_1601_epoch() {
    /* 2024-01-01T00:00:00Z is 1704067200 unix. */
    let stored = (1_704_067_200 + TO_UNIX) * 1_000_000;
    assert_eq!(expiry(stored), Some(1_704_067_200));
    /* Zero is a session cookie, not 1601. */
    assert_eq!(expiry(0), None);
}

/// A store read end to end, built here so the test does not need a browser.
#[test]
fn a_whole_store_is_read_with_its_values_decrypted() {
    let dir = std::env::temp_dir().join(format!("devpit-cookies-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let store = dir.join("Cookies");
    let _ = std::fs::remove_file(&store);

    let key = key_from(&Password::Fallback);
    {
        let db = rusqlite::Connection::open(&store).unwrap();
        db.execute_batch(
            "CREATE TABLE cookies (host_key TEXT, name TEXT, value TEXT, \
             encrypted_value BLOB, path TEXT, expires_utc INTEGER, \
             is_secure INTEGER, is_httponly INTEGER);",
        )
        .unwrap();
        db.execute(
            "INSERT INTO cookies VALUES (?1, ?2, '', ?3, '/', ?4, 1, 1)",
            rusqlite::params![
                ".example.com",
                "session",
                sealed(b"secret-session", &key, b"v10"),
                (1_704_067_200_i64 + TO_UNIX) * 1_000_000
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO cookies VALUES ('other.test', 'plain', 'not-encrypted', x'', '/a', 0, 0, 0)",
            [],
        )
        .unwrap();
    }

    let found = read(&store, &Password::Fallback).expect("the store reads");
    assert_eq!(found.len(), 2);

    let session = found.iter().find(|one| one.name == "session").unwrap();
    assert_eq!(session.value.seen(), "secret-session");
    assert_eq!(session.host, ".example.com");
    assert_eq!(session.expires, Some(1_704_067_200));
    assert!(session.secure && session.http_only);

    let plain_one = found.iter().find(|one| one.name == "plain").unwrap();
    assert_eq!(plain_one.value.seen(), "not-encrypted");
    assert_eq!(plain_one.expires, None, "zero is a session cookie");
    assert!(!plain_one.secure);

    let _ = std::fs::remove_file(&store);
}

#[test]
fn a_store_that_is_not_there_says_so_rather_than_returning_nothing() {
    let missing = std::path::Path::new("/nowhere/at/all/Cookies");
    assert!(matches!(
        read(missing, &Password::Fallback),
        Err(Refused::NoStore(_))
    ));
}
