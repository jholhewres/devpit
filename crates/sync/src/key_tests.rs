use super::*;

fn fixed(byte: u8) -> SyncKey {
    SyncKey::from_bytes([byte; 32])
}

#[test]
fn a_recovery_code_gives_back_the_key_it_was_made_from() {
    let key = SyncKey::generate().expect("a key");
    let code = key.recovery_code();
    assert_eq!(
        SyncKey::from_recovery_code(&code).expect("read").to_hex(),
        key.to_hex()
    );
    // Written by hand: lower case, spaces for dashes, a line break.
    let typed = code.to_lowercase().replace('-', " ").replacen(' ', "\n", 1);
    assert_eq!(
        SyncKey::from_recovery_code(&typed).expect("read").to_hex(),
        key.to_hex()
    );
}

#[test]
fn a_recovery_code_is_letters_read_easily_in_groups() {
    let code = fixed(7).recovery_code();
    assert!(
        code.chars().all(|c| c == '-' || LETTERS.contains(c)),
        "{code}"
    );
    assert!(!code.contains(['0', 'O', '1', 'I']));
    assert!(code.split('-').all(|group| group.len() <= GROUP));
}

#[test]
fn a_mistyped_recovery_code_is_caught_before_it_is_tried() {
    let code = fixed(9).recovery_code();
    let first = code.chars().next().expect("a letter");
    let other = if first == 'A' { 'B' } else { 'A' };
    let wrong = format!("{other}{}", &code[1..]);
    assert!(SyncKey::from_recovery_code(&wrong).is_err());
    assert!(SyncKey::from_recovery_code(&code[..code.len() - 6]).is_err());
    assert!(SyncKey::from_recovery_code(&code.replacen(first, "0", 1)).is_err());
}

#[test]
fn a_blob_name_says_nothing_of_what_it_holds_and_differs_by_key() {
    let one = fixed(1);
    let name = one.blob_name("prefs");
    assert_eq!(name.len(), 32);
    assert!(name.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(!name.contains("prefs"));
    assert_eq!(name, one.blob_name("prefs"), "stable");
    assert_ne!(name, one.blob_name("projects"));
    assert_ne!(name, fixed(2).blob_name("prefs"));
}

#[test]
fn a_key_is_told_apart_by_an_id_that_is_not_the_key() {
    let one = fixed(1);
    assert_eq!(one.id().len(), 16);
    assert_ne!(one.id(), fixed(2).id());
    assert!(!one.to_hex().contains(&one.id()));
    assert_ne!(
        one.sealing(),
        [1u8; 32],
        "nothing seals with the key itself"
    );
    assert!(
        !format!("{one:?}").contains(&one.to_hex()),
        "a debug print says the id only"
    );
}

#[test]
fn a_kept_key_reads_back_and_anything_else_is_refused() {
    let key = fixed(3);
    assert_eq!(
        SyncKey::from_hex(&key.to_hex()).expect("read").to_hex(),
        key.to_hex()
    );
    assert!(SyncKey::from_hex("not a key").is_err());
    assert!(SyncKey::from_hex(&key.to_hex()[..62]).is_err());
}
