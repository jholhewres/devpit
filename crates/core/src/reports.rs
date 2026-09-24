//! devpit's own errors, kept for an error report — only when the person asked.
//!
//! Off, [`record`] does nothing and the file does not exist: switching off
//! wipes it. On, each error is cleaned of paths as it is written, repeats fold
//! into one entry, and the file is held to [`MAX_ENTRIES`], [`MAX_BYTES`] and
//! [`MAX_AGE_SECS`] so it never grows into something to worry about.

use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError, TryLockError};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_ENTRIES: usize = 200;
pub const MAX_BYTES: usize = 256 * 1024;
pub const MAX_AGE_SECS: u64 = 15 * 24 * 60 * 60;
const MESSAGE_CAP: usize = 1024;
const STACK_CAP: usize = 8 * 1024;
/// What a path becomes. Only the home is kept as `~`, and only when it stands
/// alone: anything under it names a project, a repo or a person's folders.
const PATH: &str = "<path>";

/// One fingerprint's worth of occurrences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub fingerprint: String,
    pub kind: String,
    pub location: Option<String>,
    pub message: String,
    pub stack: Option<String>,
    pub count: u32,
    /// Seconds since the epoch.
    pub first_seen: u64,
    pub last_seen: u64,
}

/// An error as it was caught, before it is cleaned.
#[derive(Debug, Clone, Copy)]
pub struct Report<'a> {
    /// `panic`, `internal`, `background` or `frontend`.
    pub kind: &'a str,
    pub location: Option<&'a str>,
    pub message: &'a str,
    pub stack: Option<&'a str>,
}

/// The file, under a root.
#[derive(Debug, Clone)]
pub struct ErrorLog {
    path: PathBuf,
}

impl ErrorLog {
    pub fn at(root: &Path) -> Self {
        Self {
            path: root.join("logs").join("errors.jsonl"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What is kept. A line that does not parse is dropped, not fatal: the
    /// file is devpit's own, and one bad write should not cost the rest.
    pub fn read(&self) -> Vec<Entry> {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return Vec::new();
        };
        text.lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }

    /// Adds one occurrence, cleaned, folded into its fingerprint, and prunes.
    pub fn add(&self, report: Report<'_>, now: u64, home: Option<&Path>) -> std::io::Result<()> {
        self.add_clean(clean(report, home), 1, now)
    }

    fn add_clean(&self, error: Clean, times: u32, now: u64) -> std::io::Result<()> {
        let mut entries = self.read();
        fold(&mut entries, error, times, now);
        keep(&mut entries, now);
        self.write(&entries)
    }

    /// The ceilings applied with no new error to trigger them: errors are
    /// rare, and fifteen days cannot wait for the next one.
    pub fn prune(&self, now: u64) -> std::io::Result<()> {
        let mut entries = self.read();
        let before = entries.len();
        keep(&mut entries, now);
        match (before, entries.len()) {
            (0, _) => Ok(()),
            (_, 0) => self.wipe(),
            (was, now_kept) if was != now_kept => self.write(&entries),
            _ => Ok(()),
        }
    }

    /// The next errors to report: the ones waiting longest.
    pub fn next_batch(&self, most: usize) -> Vec<Entry> {
        let mut entries = self.read();
        entries.sort_by_key(|entry| entry.first_seen);
        entries.truncate(most);
        entries
    }

    /// Takes out what a report carried. An error seen again while the batch
    /// was out keeps the occurrences the report did not include.
    pub fn forget_sent(&self, sent: &[Entry]) -> std::io::Result<()> {
        let mut entries = self.read();
        for gone in sent {
            if let Some(at) = entries
                .iter()
                .position(|entry| entry.fingerprint == gone.fingerprint)
            {
                let entry = &mut entries[at];
                if entry.count <= gone.count {
                    entries.remove(at);
                } else {
                    entry.count -= gone.count;
                    entry.first_seen = gone.last_seen;
                }
            }
        }
        if entries.is_empty() {
            self.wipe()
        } else {
            self.write(&entries)
        }
    }

    /// Nothing kept: the file goes, not just its contents, and neither does a
    /// write that stopped before its rename.
    pub fn wipe(&self) -> std::io::Result<()> {
        for path in [self.temporary(), self.path.clone()] {
            match std::fs::remove_file(&path) {
                Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err),
                _ => {}
            }
        }
        Ok(())
    }

    fn temporary(&self) -> PathBuf {
        self.path.with_extension("jsonl.tmp")
    }

    /// Whole, through a temporary and a rename, so a crash mid-write leaves
    /// the old file rather than half of the new one.
    fn write(&self, entries: &[Entry]) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            crate::home::make_private_root(dir)?;
        }
        let temporary = self.temporary();
        crate::home::write_private(&temporary, lines(entries).as_bytes())?;
        std::fs::rename(&temporary, &self.path)
    }
}

/// The cleaned occurrence, as it would be stored.
struct Clean {
    fingerprint: String,
    kind: String,
    location: Option<String>,
    message: String,
    stack: Option<String>,
}

fn clean(report: Report<'_>, home: Option<&Path>) -> Clean {
    // Cut before cleaning, with room for what cleaning shortens: the text can
    // come from the window, and cleaning a hundred megabytes costs several
    // hundred.
    let kept = |text: &str, bytes: usize| cap(&sanitize(prefix(text, bytes * 4), home), bytes);
    let kind = kept(report.kind, 32);
    let location = report.location.map(|at| kept(at, 256));
    let message = kept(report.message, MESSAGE_CAP);
    let stack = report.stack.map(|stack| kept(stack, STACK_CAP));
    Clean {
        fingerprint: fingerprint(&kind, location.as_deref(), &message),
        kind,
        location,
        message,
        stack,
    }
}

fn fold(entries: &mut Vec<Entry>, error: Clean, times: u32, now: u64) {
    if let Some(seen) = entries
        .iter_mut()
        .find(|entry| entry.fingerprint == error.fingerprint)
    {
        seen.count = seen.count.saturating_add(times);
        seen.last_seen = now;
        return;
    }
    entries.push(Entry {
        fingerprint: error.fingerprint,
        kind: error.kind,
        location: error.location,
        message: error.message,
        stack: error.stack,
        count: times,
        first_seen: now,
        last_seen: now,
    });
}

/// Drops what is too old, then the least recently seen until both ceilings
/// hold.
pub fn keep(entries: &mut Vec<Entry>, now: u64) {
    entries.retain(|entry| now.saturating_sub(entry.last_seen) <= MAX_AGE_SECS);
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.last_seen));
    entries.truncate(MAX_ENTRIES);
    while !entries.is_empty() && lines(entries).len() > MAX_BYTES {
        entries.pop();
    }
}

fn lines(entries: &[Entry]) -> String {
    entries
        .iter()
        .filter_map(|entry| serde_json::to_string(entry).ok())
        .map(|line| line + "\n")
        .collect()
}

/// Same kind, same place, same message once the numbers and ids are taken
/// out: a port or a run id should not make one bug look like a hundred.
pub fn fingerprint(kind: &str, location: Option<&str>, message: &str) -> String {
    let mut hash = Sha256::new();
    for part in [kind, location.unwrap_or(""), &normalize(message)] {
        hash.update(part.as_bytes());
        hash.update([0]);
    }
    hash.finalize()
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn normalize(message: &str) -> String {
    message
        .split(' ')
        .map(|word| {
            let letters_and_digits = word.chars().filter(char::is_ascii_alphanumeric).count();
            let has_digit = word.chars().any(|c| c.is_ascii_digit());
            if has_digit && letters_and_digits >= 16 {
                "*".to_string()
            } else {
                word.chars()
                    .map(|c| if c.is_ascii_digit() { '#' } else { c })
                    .collect()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Takes every path and every secret-looking word out of a text. The home
/// standing alone becomes `~`; anything absolute, or under a home, becomes
/// [`PATH`]; a URL loses its credentials and a long word with digits in it —
/// a token, an id — becomes `*`.
pub fn sanitize(text: &str, home: Option<&Path>) -> String {
    // `file:///x` is the path `/x` behind a scheme.
    let mut text = text.replace("file://", "file:");
    if let Some(home) = home.and_then(Path::to_str).filter(|home| home.len() > 1) {
        text = home_as_tilde(&text, home);
    }
    without_secrets(&without_paths(&without_named(&text)))
}

/// What a failed command named, out: the arguments of `git <what> … failed`,
/// anything in single quotes (git and shells quote the names they mean), and
/// a long quote in backticks (the command somebody wrote). What failed stays.
fn without_named(text: &str) -> String {
    let text = git_arguments_out(text);
    let text = quoted_out(&text, '\'', 0);
    quoted_out(&text, '`', 24)
}

fn git_arguments_out(text: &str) -> String {
    text.split('\n')
        .map(|line| {
            let Some(start) = line.find("git ") else {
                return line.to_string();
            };
            let after_what = line[start + 4..].find(' ').map(|space| start + 4 + space);
            match (after_what, line.find(" failed")) {
                (Some(from), Some(to)) if from < to => {
                    format!("{} …{}", &line[..from], &line[to..])
                }
                _ => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `quote`d span longer than `longest` becomes `…`. A quote after a
/// letter is an apostrophe, not the start of one.
fn quoted_out(text: &str, quote: char, longest: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        let opens = chars[at] == quote && (at == 0 || !chars[at - 1].is_alphanumeric());
        let closes = opens
            .then(|| {
                (at + 1..chars.len()).find(|&i| {
                    chars[i] == quote && chars.get(i + 1).is_none_or(|c| !c.is_alphanumeric())
                })
            })
            .flatten()
            .filter(|&end| !chars[at + 1..end].contains(&'\n'));
        match closes {
            Some(end) if end - at - 1 > longest => {
                out.push(quote);
                out.push('…');
                out.push(quote);
                at = end + 1;
            }
            Some(end) => {
                out.extend(&chars[at..=end]);
                at = end + 1;
            }
            None => {
                out.push(chars[at]);
                at += 1;
            }
        }
    }
    out
}

/// The home only where it ends a segment: `/home/jholder` is not `/home/jhol`.
fn home_as_tilde(text: &str, home: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(home) {
        let after = rest[at + home.len()..].chars().next();
        out.push_str(&rest[..at]);
        if after.is_none_or(|c| !continues_a_name(c)) {
            out.push('~');
        } else {
            out.push_str(home);
        }
        rest = &rest[at + home.len()..];
    }
    out.push_str(rest);
    out
}

fn without_paths(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        if !starts_a_path(&chars, at) {
            out.push(chars[at]);
            at += 1;
            continue;
        }
        // Quoted, a path runs to its closing quote: `{:?}` quotes every path
        // it prints, spaces and all.
        let quote = at
            .checked_sub(1)
            .map(|before| chars[before])
            .filter(|c| matches!(c, '"' | '\'' | '`'));
        let end = (at..chars.len())
            .find(|&i| match quote {
                Some(quote) => chars[i] == quote || chars[i] == '\n',
                None => ends_a_path(chars[i]),
            })
            .unwrap_or(chars.len());
        let path: String = chars[at..end].iter().collect();
        out.push_str(if path == "~" || path == "~/" {
            "~"
        } else {
            PATH
        });
        at = end;
    }
    out
}

/// After anything that cannot be inside a name: `cwd:/x`, `(/x)`, `;/x`.
fn starts_a_path(chars: &[char], at: usize) -> bool {
    if at > 0 && (continues_a_name(chars[at - 1]) || chars[at - 1] == '/') {
        return false;
    }
    let next = |offset: usize| chars.get(at + offset).copied();
    match chars[at] {
        '/' => next(1).is_some_and(|c| !ends_a_path(c) && c != '/'),
        '~' => true,
        '\\' => next(1) == Some('\\'),
        drive if drive.is_ascii_alphabetic() => {
            next(1) == Some(':') && matches!(next(2), Some('\\') | Some('/'))
        }
        _ => false,
    }
}

fn continues_a_name(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | '~' | '\\')
}

fn ends_a_path(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '\'' | '"' | ')' | ']' | '}' | ',' | ';' | '|' | '`' | '>'
        )
}

/// URL credentials out, and any long run of letters and digits with a digit
/// in it — what a token or an id looks like, and what no sentence does.
fn without_secrets(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    for c in text.chars().chain(std::iter::once('\0')) {
        if c.is_ascii_alphanumeric() || matches!(c, '_' | '-') {
            word.push(c);
            continue;
        }
        out.push_str(if looks_secret(&word) { "*" } else { &word });
        word.clear();
        if c != '\0' {
            out.push(c);
        }
    }
    credentials_out(&out)
}

fn looks_secret(word: &str) -> bool {
    word.chars().filter(char::is_ascii_alphanumeric).count() >= 16
        && word.chars().any(|c| c.is_ascii_digit())
}

/// `scheme://user:password@host` keeps the scheme and the host.
fn credentials_out(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("://") {
        let (head, tail) = rest.split_at(at + 3);
        out.push_str(head);
        let authority_end = tail
            .find(|c: char| c == '/' || c.is_whitespace())
            .unwrap_or(tail.len());
        match tail[..authority_end].rfind('@') {
            Some(user_end) => {
                out.push('*');
                rest = &tail[user_end..];
            }
            None => rest = tail,
        }
    }
    out.push_str(rest);
    out
}

fn prefix(text: &str, bytes: usize) -> &str {
    if text.len() <= bytes {
        return text;
    }
    let mut end = bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn cap(text: &str, bytes: usize) -> String {
    if text.len() <= bytes {
        return text.to_string();
    }
    let mut end = bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// How long a repeat of the same error is only counted, not written. An
/// error in a loop that redraws sixty times a second would otherwise rewrite
/// the file sixty times a second.
pub const QUIET_SECS: u64 = 5;

/// The file and what was written to it lately.
pub struct Recorder {
    log: ErrorLog,
    /// Fingerprint → when it was last written, and repeats held since.
    lately: HashMap<String, (u64, u32)>,
}

impl Recorder {
    pub fn new(log: ErrorLog) -> Self {
        Self {
            log,
            lately: HashMap::new(),
        }
    }

    /// Writes the error, or counts it if the same one was written less than
    /// [`QUIET_SECS`] ago. The count held goes in with the next write, so a
    /// burst loses nothing but its last few seconds.
    pub fn take(
        &mut self,
        report: Report<'_>,
        now: u64,
        home: Option<&Path>,
    ) -> std::io::Result<()> {
        let error = clean(report, home);
        if let Some((written, held)) = self.lately.get_mut(&error.fingerprint) {
            if now.saturating_sub(*written) < QUIET_SECS {
                *held = held.saturating_add(1);
                return Ok(());
            }
        }
        let held = self
            .lately
            .remove(&error.fingerprint)
            .map_or(0, |(_, held)| held);
        let fingerprint = error.fingerprint.clone();
        self.log.add_clean(error, held.saturating_add(1), now)?;
        self.lately
            .retain(|_, (written, _)| now.saturating_sub(*written) < QUIET_SECS);
        self.lately.insert(fingerprint, (now, 0));
        Ok(())
    }
}

/// Where the process writes, while the person has it on. `None` is off.
static ON: Mutex<Option<Recorder>> = Mutex::new(None);

thread_local! {
    /// Set while this thread is recording, so an error raised by recording —
    /// or a panic in the middle of it — does not record itself for ever.
    static RECORDING: Cell<bool> = const { Cell::new(false) };
}

/// This thread recording, until dropped — by a return or by a panic.
struct Recording;

impl Recording {
    fn enter() -> Option<Self> {
        (!RECORDING.with(|recording| recording.replace(true))).then_some(Self)
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        RECORDING.with(|recording| recording.set(false));
    }
}

/// Turns recording on under `root`, or off. Off wipes what was kept.
pub fn switch(root: &Path, on: bool) -> std::io::Result<()> {
    let log = ErrorLog::at(root);
    let mut current = ON.lock().unwrap_or_else(PoisonError::into_inner);
    if on {
        let pruned = log.prune(now());
        *current = Some(Recorder::new(log));
        pruned
    } else {
        *current = None;
        log.wipe()
    }
}

/// Keeps one error, if the person asked for that. Never fails loudly: a
/// report that could not be written is not worth a second error.
pub fn record(report: Report<'_>) {
    let Some(_recording) = Recording::enter() else {
        return;
    };
    // A panic mid-write poisons the lock; the recorder holds nothing a
    // half-finished write could have broken, so recording goes on.
    let mut current = ON.lock().unwrap_or_else(PoisonError::into_inner);
    take(current.as_mut(), report);
}

/// [`record`] for a panic: never waits, because the thread holding the lock
/// may be the one going down.
pub fn record_now_or_never(report: Report<'_>) {
    let Some(_recording) = Recording::enter() else {
        return;
    };
    let mut current = match ON.try_lock() {
        Ok(current) => current,
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
        Err(TryLockError::WouldBlock) => return,
    };
    take(current.as_mut(), report);
}

fn take(recorder: Option<&mut Recorder>, report: Report<'_>) {
    if let Some(recorder) = recorder {
        let _ = recorder.take(report, now(), dirs::home_dir().as_deref());
    }
}

/// The log being written to, if the person has reports on. The lock is held
/// for `then`, so nothing is recorded between reading a batch and forgetting it.
pub fn with_current<T>(then: impl FnOnce(&ErrorLog) -> T) -> Option<T> {
    let current = ON.lock().unwrap_or_else(PoisonError::into_inner);
    current.as_ref().map(|recorder| then(&recorder.log))
}

/// Seconds since the epoch as RFC 3339, in UTC — what the server reads.
pub fn rfc3339(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    // Days to a civil date, after Howard Hinnant's `civil_from_days`.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

/// Keeps panics too, after whatever hook was there has had its say — the
/// message on stderr is still how a person running from a terminal sees one.
pub fn keep_panics() {
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        before(info);
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("a panic with no message");
        let message = panic_message(message);
        let location = info.location().map(ToString::to_string);
        let stack = std::backtrace::Backtrace::force_capture().to_string();
        record_now_or_never(Report {
            kind: "panic",
            location: location.as_deref(),
            message: &message,
            stack: Some(&stack),
        });
    }));
}

/// The shape of a panic without the value it quoted. std puts the value in
/// the message — the string that was sliced, the `Err` that was unwrapped, the
/// two sides of an assertion — and that value can be anybody's text.
pub fn panic_message(payload: &str) -> String {
    let mut message = payload.lines().next().unwrap_or_default().to_string();
    if let Some(at) = message.find(" of `") {
        message.truncate(at);
    }
    if let Some(at) = message.find(" value: ") {
        message.truncate(at + " value: ".len());
        message.push('…');
    }
    // What is left in backticks is std naming code (`Result::unwrap()`), or a
    // value this does not know the form of; only the short one is code.
    let mut out = String::with_capacity(message.len());
    for (index, part) in message.split('`').enumerate() {
        if index > 0 {
            out.push('`');
        }
        let quoted = index % 2 == 1;
        out.push_str(if quoted && part.len() > 24 {
            "…"
        } else {
            part
        });
    }
    out
}

/// A routine that runs by itself failed: said on stderr as before, and kept.
pub fn background(what: &str, err: &dyn std::fmt::Display) {
    let message = err.to_string();
    eprintln!("{what}: {message}");
    record(Report {
        kind: "background",
        location: Some(what),
        message: &message,
        stack: None,
    });
}

#[cfg(test)]
#[path = "reports_tests.rs"]
mod tests;
