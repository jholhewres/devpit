//! The code that pairs a device: shown on this machine's screen, good once,
//! for two minutes, and burned after a few wrong guesses.
//!
//! Presence is the point. Pairing gives a device the machine, so it needs
//! someone at the machine to read the code off its screen.

use std::sync::Mutex;

/// How long a code is good for, in seconds.
pub(crate) const GOOD_FOR: f64 = 120.0;

/// Wrong guesses before the code is burned.
const GUESSES: u32 = 5;

/// No 0/O or 1/I: read off a screen and typed on a phone.
const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Offered {
    pub code: String,
    pub expires_at: f64,
    wrong: u32,
}

static OFFERED: Mutex<Option<Offered>> = Mutex::new(None);

/// A code of eight, from random bytes.
pub(crate) fn code_of(bytes: &[u8; 8]) -> String {
    bytes
        .iter()
        .map(|byte| ALPHABET[usize::from(*byte) % ALPHABET.len()] as char)
        .collect()
}

/// A new code, in place of any before it.
pub(crate) fn offer(now: f64) -> Option<Offered> {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).ok()?;
    let offered = Offered {
        code: code_of(&bytes),
        expires_at: now + GOOD_FOR,
        wrong: 0,
    };
    *OFFERED.lock().ok()? = Some(offered.clone());
    Some(offered)
}

/// Whether `code` is the one offered, still good: taken if so, and burned
/// after too many that were not.
pub(crate) fn take(code: &str, now: f64) -> bool {
    let Ok(mut held) = OFFERED.lock() else {
        return false;
    };
    taken(&mut held, code, now)
}

pub(crate) fn taken(held: &mut Option<Offered>, code: &str, now: f64) -> bool {
    let Some(offered) = held.as_mut() else {
        return false;
    };
    if now > offered.expires_at {
        *held = None;
        return false;
    }
    let typed = code.trim().to_ascii_uppercase().replace(['-', ' '], "");
    if typed == offered.code {
        *held = None;
        return true;
    }
    offered.wrong += 1;
    if offered.wrong >= GUESSES {
        *held = None;
    }
    false
}

/// Puts the offered code away: paired, or the person closed it.
pub(crate) fn withdraw() {
    if let Ok(mut held) = OFFERED.lock() {
        *held = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{code_of, taken, Offered, GUESSES};

    fn offered(code: &str) -> Option<Offered> {
        Some(Offered {
            code: code.to_owned(),
            expires_at: 100.0,
            wrong: 0,
        })
    }

    #[test]
    fn a_code_is_eight_letters_read_easily() {
        let code = code_of(&[0, 1, 2, 31, 32, 200, 255, 7]);
        assert_eq!(code.len(), 8);
        assert!(!code.contains(['0', 'O', '1', 'I']));
    }

    #[test]
    fn a_code_pairs_once_while_it_is_good() {
        let mut held = offered("ABCD2345");
        assert!(taken(&mut held, "abcd-2345", 50.0));
        assert!(!taken(&mut held, "ABCD2345", 50.0), "only once");

        let mut late = offered("ABCD2345");
        assert!(!taken(&mut late, "ABCD2345", 101.0));
    }

    #[test]
    fn too_many_wrong_guesses_burn_it() {
        let mut held = offered("ABCD2345");
        for _ in 0..GUESSES {
            assert!(!taken(&mut held, "WRONG000", 10.0));
        }
        assert!(!taken(&mut held, "ABCD2345", 10.0));
    }
}
