use super::*;

const SAID: &[u8] = br#"{"appearance.theme":"light"}"#;

fn key(byte: u8) -> SyncKey {
    SyncKey::from_bytes([byte; 32])
}

#[test]
fn a_sealed_blob_opens_with_its_key_name_and_version() {
    let sealed = seal(&key(1), "abc", 3, SAID).expect("seal");
    assert_eq!(open(&key(1), "abc", 3, &sealed).expect("open"), SAID);
}

#[test]
fn a_sealed_blob_does_not_carry_what_it_holds() {
    let sealed = seal(&key(1), "abc", 3, SAID).expect("seal");
    assert!(!sealed.windows(5).any(|window| window == b"light"));
    assert!(!sealed.windows(10).any(|window| window == b"appearance"));
}

#[test]
fn another_key_opens_nothing_and_says_so() {
    let sealed = seal(&key(1), "abc", 3, SAID).expect("seal");
    assert_eq!(open(&key(2), "abc", 3, &sealed), Err(Unsealed::OtherKey));
    // A key that lies about its id still cannot open it.
    let mut lying = sealed.clone();
    lying[MAGIC.len()..MAGIC.len() + ID].copy_from_slice(&key(2).id_bytes());
    assert_eq!(open(&key(2), "abc", 3, &lying), Err(Unsealed::Tampered));
}

#[test]
fn a_blob_served_as_another_name_or_version_does_not_open() {
    let sealed = seal(&key(1), "abc", 3, SAID).expect("seal");
    assert_eq!(open(&key(1), "abd", 3, &sealed), Err(Unsealed::Tampered));
    assert_eq!(open(&key(1), "abc", 4, &sealed), Err(Unsealed::Tampered));
    assert_eq!(open(&key(1), "abc", 2, &sealed), Err(Unsealed::Tampered));
}

#[test]
fn a_changed_byte_anywhere_does_not_open() {
    let sealed = seal(&key(1), "abc", 3, SAID).expect("seal");
    for at in [MAGIC.len() + ID, HEADER, sealed.len() - 1] {
        let mut changed = sealed.clone();
        changed[at] ^= 1;
        assert_eq!(
            open(&key(1), "abc", 3, &changed),
            Err(Unsealed::Tampered),
            "byte {at}"
        );
    }
    assert_eq!(open(&key(1), "abc", 3, b"plain"), Err(Unsealed::NotSealed));
}

#[test]
fn the_same_words_seal_differently_every_time() {
    let one = seal(&key(1), "abc", 3, SAID).expect("seal");
    let two = seal(&key(1), "abc", 3, SAID).expect("seal");
    assert_ne!(one, two, "a nonce is drawn for every seal");
    assert_eq!(sealed_by(&one), Some(key(1).id_bytes()));
}
