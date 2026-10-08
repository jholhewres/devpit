//! Text that looks like a secret, taken out before devpit keeps the text.
//!
//! Keys pasted into a chat ended up in transcripts, and a note or a card is
//! read by every session after. The shapes here are the common ones — API
//! keys by their prefixes, AWS access keys, JWTs, private keys — not a proof:
//! a secret with no shape passes, which is why the vault exists.

/// What replaces a secret.
const REMOVED: &str = "[secret removed]";

/// Prefixes that start a key, and the shortest a key with one is.
const PREFIXED: [(&str, usize); 10] = [
    ("sk-", 20),
    ("sk_live_", 20),
    ("rk_live_", 20),
    ("ghp_", 30),
    ("gho_", 30),
    ("github_pat_", 30),
    ("hf_", 20),
    ("xoxb-", 20),
    ("xoxp-", 20),
    ("glpat-", 20),
];

/// Whether one word is a secret by its shape.
fn secret(word: &str) -> bool {
    let word = word.trim_matches(|c: char| matches!(c, '"' | '\'' | '`' | ',' | ';' | '(' | ')'));
    if PREFIXED
        .iter()
        .any(|(prefix, shortest)| word.starts_with(prefix) && word.len() >= *shortest)
    {
        return true;
    }
    let aws = word.len() == 20
        && (word.starts_with("AKIA") || word.starts_with("ASIA"))
        && word
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    let jwt = word.starts_with("eyJ") && word.matches('.').count() == 2 && word.len() > 40;
    aws || jwt
}

/// `text` with every secret-shaped word and private key replaced, and how
/// many were.
pub(crate) fn redacted(text: &str) -> (String, usize) {
    let mut count = 0;
    let mut out = String::with_capacity(text.len());
    let mut in_key = false;
    for line in text.split_inclusive('\n') {
        if line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----") {
            in_key = true;
            count += 1;
            out.push_str(REMOVED);
            out.push('\n');
            continue;
        }
        if in_key {
            in_key = !line.contains("-----END");
            continue;
        }
        let mut kept = String::with_capacity(line.len());
        for piece in line.split_inclusive(char::is_whitespace) {
            let word = piece.trim_end();
            // `NAME=value` keeps its name: the name says which secret it was.
            let (name, value) = match word.rsplit_once('=') {
                Some((name, value)) => (&word[..=name.len()], value),
                None => ("", word),
            };
            if secret(value) {
                count += 1;
                kept.push_str(name);
                kept.push_str(REMOVED);
                kept.push_str(&piece[word.len()..]);
            } else {
                kept.push_str(piece);
            }
        }
        out.push_str(&kept);
    }
    (out, count)
}

#[cfg(test)]
#[path = "secret_scan_tests.rs"]
mod tests;
