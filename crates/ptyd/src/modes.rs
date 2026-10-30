//! The DEC private modes a pane is in, so a client attaching late is put back
//! into them before it sees the tail: a vim in the alternate screen replayed
//! without `?1049h` draws over the shell's scrollback, and one with mouse
//! reporting off ignores every click.

/// Modes put back on attach, in this order. 1 is application cursor keys
/// (arrows in vim and less), 1049/1047/47 the alternate screen, 1000–1006 and
/// 1015/1005 mouse reporting and its encodings, 2004 bracketed paste.
const RESTORED: &[u16] = &[1, 47, 1047, 1049, 1000, 1002, 1003, 1005, 1006, 1015, 2004];
/// Cursor visibility. On by default, so it is put back only when hidden.
const CURSOR: u16 = 25;
/// The longest unfinished `ESC [ ? …` kept across a chunk boundary.
const MOST_CARRIED: usize = 64;

#[derive(Debug, Default, Clone)]
pub struct Modes {
    on: Vec<u16>,
    cursor_hidden: bool,
    carried: Vec<u8>,
}

impl Modes {
    pub fn new() -> Modes {
        Modes::default()
    }

    /// Reads `chunk` for `ESC [ ? n ; n … h|l`, across chunk boundaries.
    pub fn feed(&mut self, chunk: &[u8]) {
        if self.carried.is_empty() && !chunk.contains(&0x1b) {
            return;
        }
        let mut input = std::mem::take(&mut self.carried);
        input.extend_from_slice(chunk);
        let mut at = 0;
        while let Some(found) = input[at..].iter().position(|b| *b == 0x1b) {
            let start = at + found;
            let rest = &input[start..];
            match private_mode(rest) {
                Parsed::Done { len, set, numbers } => {
                    for number in numbers {
                        self.apply(number, set);
                    }
                    at = start + len;
                }
                Parsed::Unfinished => {
                    if rest.len() <= MOST_CARRIED {
                        self.carried = rest.to_vec();
                    }
                    return;
                }
                Parsed::Other => at = start + 1,
            }
        }
    }

    fn apply(&mut self, number: u16, set: bool) {
        if number == CURSOR {
            self.cursor_hidden = !set;
            return;
        }
        if !RESTORED.contains(&number) {
            return;
        }
        self.on.retain(|on| *on != number);
        if set {
            self.on.push(number);
        }
    }

    /// The sequence that puts a fresh terminal into these modes, or nothing.
    pub fn restore(&self) -> Vec<u8> {
        let on: Vec<String> = RESTORED
            .iter()
            .filter(|mode| self.on.contains(mode))
            .map(u16::to_string)
            .collect();
        let mut out = Vec::new();
        if !on.is_empty() {
            out.extend_from_slice(format!("\x1b[?{}h", on.join(";")).as_bytes());
        }
        if self.cursor_hidden {
            out.extend_from_slice(b"\x1b[?25l");
        }
        out
    }
}

enum Parsed {
    Done {
        len: usize,
        set: bool,
        numbers: Vec<u16>,
    },
    Unfinished,
    Other,
}

/// `rest` starts at an ESC: is it a DEC private mode set or reset?
fn private_mode(rest: &[u8]) -> Parsed {
    let Some(prefix) = rest.get(..3) else {
        return if b"\x1b[?".starts_with(rest) {
            Parsed::Unfinished
        } else {
            Parsed::Other
        };
    };
    if prefix != b"\x1b[?" {
        return Parsed::Other;
    }
    for (index, byte) in rest.iter().enumerate().skip(3) {
        match byte {
            b'0'..=b'9' | b';' => continue,
            b'h' | b'l' => {
                let numbers = std::str::from_utf8(&rest[3..index])
                    .unwrap_or_default()
                    .split(';')
                    .filter_map(|n| n.parse().ok())
                    .collect();
                return Parsed::Done {
                    len: index + 1,
                    set: *byte == b'h',
                    numbers,
                };
            }
            _ => return Parsed::Other,
        }
    }
    Parsed::Unfinished
}

#[cfg(test)]
#[path = "modes_tests.rs"]
mod tests;
