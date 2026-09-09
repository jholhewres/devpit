//! What a terminal says about itself, read out of the byte stream.
//!
//! Programs announce their working directory, their title, where a prompt
//! begins and what a command exited with — as OSC escape sequences mixed into
//! ordinary output. Reading them is the difference between "there is text
//! moving" and "the build finished, and it failed".
//!
//! This runs in Rust rather than in the terminal widget for the reason written
//! at the top of [`crate`]: recognition has to work with the window closed. A
//! parser living in the webview is a parser that is asleep exactly when a
//! notification would be worth sending.
//!
//! Three things make it cheap enough to sit on the hot path, all three learnt
//! from Orca's `terminal-osc-cwd-title-scanner.ts`:
//!
//! 1. **A pre-filter.** A chunk with no introducer in it pays one `memchr` and
//!    nothing else. Orca measured the walks it skips as a share of a 2.2×
//!    ingest regression, which is what a flood costs when every byte is
//!    examined twice.
//! 2. **A carried tail.** A sequence is split across reads often — here more
//!    often than most, because frames are coalesced on a 16ms boundary that
//!    knows nothing about escape sequences. The unterminated end of a chunk is
//!    kept and stitched onto the next one.
//! 3. **No regular expressions.** A paste of a megabyte must not become a
//!    backtracking search.

/// The most bytes of an unfinished sequence carried to the next chunk.
///
/// A real OSC is far shorter than this; a title is a line and a cwd is a path.
/// The ceiling exists because the bytes come from a process that can write
/// anything, and an introducer that is never terminated would otherwise be a
/// buffer that grows until the machine stops.
pub const MOST_CARRIED: usize = 4096;

/// Something the terminal said about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Told {
    /// OSC 7. Where the shell is now.
    Cwd(String),
    /// OSC 0 or 2. What the program calls itself.
    Title(String),
    /// OSC 133;A. A new prompt is being drawn.
    PromptBegan,
    /// OSC 133;C. The command is running and this is its output.
    OutputBegan,
    /// OSC 133;D. The command finished, and this is what with.
    ///
    /// `None` when the shell reported no code, which some do for an empty
    /// line. A missing code is not a zero, and reporting it as one would
    /// paint a green mark for a command nobody ran.
    CommandEnded { code: Option<i32> },
    /// OSC 52. A program asked for something to be put on the clipboard,
    /// which is how copying works inside `nvim`, `fzf` and `tmux` over ssh.
    Clipboard(String),
}

/// Reads OSC sequences out of a byte stream, across chunk boundaries.
#[derive(Debug, Default)]
pub struct Scanner {
    /// The unterminated end of the last chunk, waiting for its terminator.
    carried: Vec<u8>,
}

impl Scanner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds a chunk, calling `heard` for each complete sequence in it.
    pub fn scan(&mut self, chunk: &[u8], mut heard: impl FnMut(Told)) {
        // The pre-filter. With nothing carried and no introducer here, the
        // only thing that could matter is a lone ESC at the very end, which is
        // the first half of an introducer split across the boundary.
        if self.carried.is_empty() && !contains_introducer(chunk) {
            self.carried = match chunk.last() {
                Some(0x1b) => vec![0x1b],
                _ => Vec::new(),
            };
            return;
        }

        let mut input = std::mem::take(&mut self.carried);
        input.extend_from_slice(chunk);

        let mut rest = &input[..];
        loop {
            let Some(start) = find_introducer(rest) else {
                // Nothing left that could open a sequence. A trailing ESC is
                // still worth keeping for the same reason as above.
                self.carried = match rest.last() {
                    Some(0x1b) => vec![0x1b],
                    _ => Vec::new(),
                };
                return;
            };
            let body_at = &rest[start + 2..];
            match terminator(body_at) {
                Some((end, after)) => {
                    if let Some(told) = read(&body_at[..end]) {
                        heard(told);
                    }
                    rest = &body_at[after..];
                }
                None => {
                    // Unfinished. Carry it, bounded — and when it is past the
                    // ceiling, drop it rather than keep growing: a sequence
                    // this long is not one any shell writes.
                    let unfinished = &rest[start..];
                    self.carried = if unfinished.len() <= MOST_CARRIED {
                        unfinished.to_vec()
                    } else {
                        Vec::new()
                    };
                    return;
                }
            }
        }
    }
}

fn contains_introducer(bytes: &[u8]) -> bool {
    find_introducer(bytes).is_some()
}

/// Where the next `ESC ]` begins.
///
/// A chunk ending in a bare `ESC` has no window to match here; the caller
/// keeps that byte for the next chunk instead.
fn find_introducer(bytes: &[u8]) -> Option<usize> {
    bytes.windows(2).position(|pair| pair == [0x1b, b']'])
}

/// The end of a sequence body: where it stops, and where the next byte is.
///
/// Both terminators are accepted because both are used in the wild: `BEL` by
/// most shells, `ESC \` (ST) by the specification and by `tmux`.
fn terminator(body: &[u8]) -> Option<(usize, usize)> {
    for (i, byte) in body.iter().enumerate() {
        match byte {
            0x07 => return Some((i, i + 1)),
            0x1b if body.get(i + 1) == Some(&b'\\') => return Some((i, i + 2)),
            _ => {}
        }
    }
    None
}

/// One sequence body — everything between `ESC ]` and its terminator.
fn read(body: &[u8]) -> Option<Told> {
    let text = std::str::from_utf8(body).ok()?;
    let (code, rest) = text.split_once(';').unwrap_or((text, ""));
    match code {
        // The title. Both codes set it; 0 also sets the icon name, which is
        // not a thing this product draws.
        "0" | "2" => Some(Told::Title(rest.to_owned())),
        "7" => path_of(rest).map(Told::Cwd),
        "52" => {
            // `52 ; <selection> ; <base64>`. The selection is which clipboard,
            // and this product has one.
            let (_, payload) = rest.split_once(';')?;
            Some(Told::Clipboard(payload.to_owned()))
        }
        "133" => match rest.split(';').next()? {
            "A" => Some(Told::PromptBegan),
            "C" => Some(Told::OutputBegan),
            "D" => Some(Told::CommandEnded {
                code: rest.split(';').nth(1).and_then(|code| code.parse().ok()),
            }),
            // `B` marks the end of the prompt, which is where typing starts.
            // Nothing here draws that, and a variant nothing draws is one the
            // naming guard is right to object to.
            _ => None,
        },
        _ => None,
    }
}

/// The path out of an OSC 7 `file://` URI.
///
/// The host part is dropped rather than checked. A shell on this machine
/// writes its own hostname, one inside a container writes the container's, and
/// refusing the second would turn a working cwd into a missing one for anybody
/// working in Docker.
fn path_of(uri: &str) -> Option<String> {
    let after_scheme = uri.strip_prefix("file://")?;
    let path = match after_scheme.find('/') {
        Some(slash) => &after_scheme[slash..],
        // `file://` with no path at all says nothing.
        None => return None,
    };
    Some(percent_decoded(path))
}

/// `%20` and friends, because a path with a space in it arrives encoded.
fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let pair = std::str::from_utf8(&bytes[i + 1..i + 3])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok());
            if let Some(byte) = pair {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    // Lossy: a path that is not UTF-8 must still name a directory rather than
    // take the whole sequence down with it.
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
#[path = "osc_tests.rs"]
mod tests;
