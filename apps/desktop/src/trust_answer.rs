//! The trust question answered for the person, where they already said yes.
//!
//! devpit writes the trust down before a session opens (`folder_trust`); this
//! is for a CLI that asks anyway — an account devpit does not know, a new
//! format. It watches only a session devpit just typed into a terminal of a
//! project it has, so the folder is one the person added. And only the plain
//! question: a folder that brings its own permissions or hooks asks
//! "continue without these permissions", and that is left for the person.

use std::time::Duration;

use devpit_tmux::Key;

const YES: [&str; 2] = ["Yes, I trust this folder", "Yes, proceed"];
const CURSOR: char = '❯';
/// How long a new session is watched: past it the CLI is up or the person is.
const TRIES: u32 = 120;
const EVERY: Duration = Duration::from_millis(500);
/// Arrow presses before giving up on reaching "Yes".
const MOVES: u32 = 4;

/// Where the cursor is against "Yes" on a plain trust question.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Cursor {
    OnYes,
    Above,
    Below,
}

/// The trust question on `screen`, if it is the plain one.
pub(crate) fn plain_trust(screen: &str) -> Option<Cursor> {
    let lines: Vec<&str> = screen.lines().collect();
    let lines = &lines[lines.len().saturating_sub(40)..];
    if lines
        .iter()
        .any(|line| line.contains("continue without these permissions"))
    {
        return None;
    }
    if !lines.iter().any(|line| line.contains("No, exit")) {
        return None;
    }
    let yes = lines
        .iter()
        .position(|line| YES.iter().any(|said| line.contains(said)))?;
    let cursor = lines
        .iter()
        .position(|line| line.trim_start().starts_with(CURSOR))?;
    Some(match cursor.cmp(&yes) {
        std::cmp::Ordering::Equal => Cursor::OnYes,
        std::cmp::Ordering::Less => Cursor::Above,
        std::cmp::Ordering::Greater => Cursor::Below,
    })
}

/// Watches the terminal `target` for a while and says yes to the plain trust
/// question once, if it comes.
pub(crate) fn watch(target: String) {
    let on = crate::projects::store()
        .map(|store| crate::agent_choice::trust_on(&store))
        .unwrap_or(false);
    if !on {
        return;
    }
    std::thread::spawn(move || {
        let Ok(server) = crate::sessions::tmux_server() else {
            return;
        };
        let mut moves = 0;
        for _ in 0..TRIES {
            let asked = crate::live_sessions::screen_of(&target)
                .as_deref()
                .and_then(plain_trust);
            let key = match asked {
                None => None,
                Some(Cursor::OnYes) => {
                    let _ = server.press(&target, &[Key::Enter]);
                    return;
                }
                Some(Cursor::Above) => Some(Key::Down),
                Some(Cursor::Below) => Some(Key::Up),
            };
            if let Some(key) = key {
                if moves == MOVES {
                    return;
                }
                moves += 1;
                let _ = server.press(&target, &[key]);
            }
            std::thread::sleep(EVERY);
        }
    });
}

#[cfg(test)]
#[path = "trust_answer_tests.rs"]
mod tests;
