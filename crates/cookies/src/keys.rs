//! Where Chromium's password comes from on this machine.
//!
//! Chromium encrypts its cookies with a key stretched from a password, and
//! that password lives in the desktop keyring — unless no keyring answered
//! when the profile was made, in which case Chromium uses the literal string
//! `peanuts` and the cookies are protected by file permissions alone.
//!
//! **Both cases are ordinary and the difference matters to a person**, which
//! is why [`password_for`] reports which one it found rather than quietly
//! returning a key. A profile encrypted under a keyring secret will not
//! decrypt with the fallback, and the failure — every value refusing at once —
//! looks identical to a corrupt store unless the screen can say "this machine
//! has no way to read your keyring".
//!
//! **No D-Bus client here.** `secret-tool` is one process with three
//! arguments, and `reveal.rs` already states the rule this follows: a
//! dependency that wraps that is a dependency to keep updated for nothing. A
//! machine without `secret-tool` is reported as such, with the package to
//! install, rather than being silently handed the fallback and left wondering
//! why the import found nothing.

use std::path::Path;
use std::process::Command;

use crate::chromium::Password;

/// What was found, and where — the sentence the screen needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// The keyring answered.
    Keyring,
    /// No keyring was consulted because this profile never used one.
    Fallback,
    /// A keyring holds this profile's password and this machine cannot ask it.
    Unreachable { why: String },
}

/// The directories a trusted binary may come from.
///
/// Not `$PATH`: this runs a program to obtain a decryption key, and a path
/// somebody else can prepend to is a program somebody else chose. The same
/// rule `update_deb.rs` states for the command it shows.
pub const TRUSTED: [&str; 4] = ["/usr/bin", "/bin", "/usr/local/bin", "/opt/homebrew/bin"];

fn found_in(dirs: &[&str], tool: &str) -> Option<String> {
    dirs.iter()
        .map(|dir| Path::new(dir).join(tool))
        .find(|path| path.is_file())
        .map(|path| path.display().to_string())
}

/// The keyring attribute each Chromium-family browser stores its password
/// under. Taken from what each one actually writes, not guessed from its name.
pub fn keyring_name(family: &str) -> &'static str {
    match family {
        name if name.contains("Brave") => "brave",
        name if name.contains("Edge") => "microsoft-edge",
        name if name.contains("Vivaldi") => "vivaldi",
        name if name.contains("Chromium") => "chromium",
        _ => "chrome",
    }
}

/// Asks the keyring for a browser's password.
///
/// `None` when the tool is not here or the keyring has nothing — both of which
/// are answers, not failures.
fn ask_keyring(dirs: &[&str], application: &str) -> Option<String> {
    let tool = found_in(dirs, "secret-tool")?;
    let out = Command::new(tool)
        .args(["lookup", "application", application])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let said = String::from_utf8(out.stdout).ok()?;
    /* An empty answer is no answer: secret-tool exits zero and prints nothing
    when the attribute matches no entry. */
    (!said.is_empty()).then_some(said)
}

/// The password to decrypt a browser's cookies with, and where it came from.
///
/// Takes the directories rather than reading them so the decision can be
/// tested without a keyring and without a process.
pub fn password_for(dirs: &[&str], family: &str) -> (Password, Found) {
    let application = keyring_name(family);
    if let Some(secret) = ask_keyring(dirs, application) {
        return (Password::Keyring(secret), Found::Keyring);
    }
    if found_in(dirs, "secret-tool").is_none() {
        /* The tool is missing. The profile may still be a fallback one, so
        the caller tries — but if every value refuses, this is the sentence
        that explains it instead of "corrupt store". */
        return (
            Password::Fallback,
            Found::Unreachable {
                why: "secret-tool is not installed, so this machine cannot read the desktop \
                      keyring. Install libsecret-tools to import from a profile that uses one."
                    .to_owned(),
            },
        );
    }
    (Password::Fallback, Found::Fallback)
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
