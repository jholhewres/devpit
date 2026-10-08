use super::redacted;

#[test]
fn keys_by_their_shape_are_taken_out_and_the_rest_kept() {
    let (kept, count) = redacted(
        "set JEV_API_KEY=sk-abcdefghijklmnopqrstuvwx and ran it\nAWS key AKIAIOSFODNN7EXAMPLE; hf_abcdefghijklmnopqrstu\n",
    );
    assert_eq!(count, 3);
    assert_eq!(
        kept,
        "set JEV_API_KEY=[secret removed] and ran it\nAWS key [secret removed] [secret removed]\n"
    );
}

#[test]
fn a_private_key_goes_whole() {
    let (kept, count) = redacted("before\n-----BEGIN OPENSSH PRIVATE KEY-----\nabc\ndef\n-----END OPENSSH PRIVATE KEY-----\nafter\n");
    assert_eq!(count, 1);
    assert_eq!(kept, "before\n[secret removed]\nafter\n");
}

#[test]
fn ordinary_words_that_share_a_prefix_stay() {
    let text = "sk-learn is a library; the task-runner and hf_ are words; eyJ alone too\n";
    assert_eq!(redacted(text), (text.to_owned(), 0));
}
