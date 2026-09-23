//! A pane's stream, cut into commands.
//!
//! What a block is: the line somebody ran, where they ran it, when, what it
//! printed, and how it ended. The shell hooks say where each part begins —
//! `OSC 133;C` as output starts, `133;D` as it ends, devpit's own `777` line
//! with the command, `OSC 7` with the folder — and this cuts the bytes
//! between them out of the raw stream.
//!
//! Raw, because that is the only place the output exists whole. The attached
//! client sees tmux redrawing a screen; the tap sees what the program wrote,
//! in order, which is what a block has to show.

use std::collections::VecDeque;

use crate::osc::{Scanner, Span, Told};

/// The most output kept for one block. Past it the start goes and the end
/// stays: the end of a build is where the error is.
pub const MOST_OUTPUT: usize = 1024 * 1024;

/// A command's facts, without its output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    /// Rises with every block this pane has had, across restarts too.
    pub id: u64,
    pub command: Option<String>,
    pub cwd: Option<String>,
    /// Unix milliseconds.
    pub started_at: u64,
    pub ended_at: Option<u64>,
    /// `None` while running, and for a command that ended without saying.
    pub code: Option<i32>,
    /// It took the whole screen — an editor, a pager, an agent's TUI — so its
    /// output is a screen's worth of drawing, not lines to read back.
    pub interactive: bool,
    /// The start of its output was dropped to keep the end.
    pub truncated: bool,
    /// Marked by the person, to find it again in a long list.
    pub bookmarked: bool,
}

/// One command, as far as it has got.
#[derive(Debug, Clone)]
pub struct Block {
    pub head: Head,
    pub output: Vec<u8>,
}

/// What cutting a chunk changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cut {
    Started(Head),
    /// The running block learnt something new about itself: it took the whole
    /// screen.
    Changed(Head),
    Ended(Head),
}

/// tmux's passthrough envelope around a sequence: `ESC P tmux; ESC` before
/// the inner `ESC ]`, and `ESC \` after its terminator. Part of the sequence,
/// so it is cut with it rather than left in the output as stray bytes.
const OPENS: &[u8] = b"\x1bPtmux;\x1b";
const CLOSES: &[u8] = b"\x1b\\";

/// The alternate screen being entered: a full-screen program.
const ALT_SCREEN: &[u8] = b"\x1b[?1049h";

/// Cuts one pane's stream into blocks.
#[derive(Debug, Default)]
pub struct Segmenter {
    scanner: Scanner,
    cwd: Option<String>,
    /// The line announced for the command about to start.
    announced: Option<String>,
    running: Option<Block>,
    next_id: u64,
    /// The end of the last chunk, not yet known to be output: the start of a
    /// sequence the scanner is still waiting to finish, or of tmux's envelope
    /// around one. The next chunk decides.
    held: Vec<u8>,
    /// Where in `held` that unfinished sequence begins.
    held_sequence: Option<usize>,
    /// The rest of an envelope's closing `ESC \`, when a chunk ended inside it.
    expect_close: &'static [u8],
    /// A prompt has been seen: the shell's hooks are in place, so its commands
    /// can be shown as blocks at all.
    integrated: bool,
    /// The last thing heard was a prompt, with no command since.
    at_prompt: bool,
    /// The running block turned interactive during the last feed.
    changed: bool,
}

impl Segmenter {
    pub fn new() -> Self {
        Self::default()
    }

    /// The block still running, if one is.
    pub fn running(&self) -> Option<&Block> {
        self.running.as_ref()
    }

    /// Carries on numbering after blocks kept from before a restart, so an
    /// id never names two blocks.
    pub fn resume_after(&mut self, id: u64) {
        self.next_id = self.next_id.max(id);
    }

    /// Whether this pane's shell has been heard marking its prompts.
    pub fn integrated(&self) -> bool {
        self.integrated
    }

    /// Whether the shell is at its prompt, waiting for a line.
    pub fn at_prompt(&self) -> bool {
        self.at_prompt
    }

    /// The folder the shell last said it was in.
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// Feeds a chunk of the raw stream. `told` hears every sequence as the
    /// plain scanner would; `cut` hears blocks starting and ending, the ended
    /// ones handed over whole.
    pub fn feed(
        &mut self,
        chunk: &[u8],
        now_ms: u64,
        mut told: impl FnMut(Told),
        mut cut: impl FnMut(Cut, Option<Block>),
    ) {
        let mut spans: Vec<(Told, Span)> = Vec::new();
        self.scanner
            .scan_spans(chunk, |one, span| spans.push((one, span)));

        // What was held, then this chunk, as one run of bytes.
        let mut data = std::mem::take(&mut self.held);
        let before = data.len();
        let continued = self.held_sequence.take();
        data.extend_from_slice(chunk);

        let mut cursor = 0;
        let expected = std::mem::take(&mut self.expect_close);
        if data.starts_with(expected) {
            cursor = expected.len();
        } else if !data.is_empty() && expected.starts_with(&data) {
            // Still inside it: this whole chunk is more of the same close.
            cursor = data.len();
            self.expect_close = &expected[data.len()..];
        }

        for (one, span) in spans {
            let start = match span.start {
                Some(at) => before + at,
                None => continued.unwrap_or(0),
            };
            let start = widened_start(&data, start).max(cursor);
            self.keep(&data[cursor.min(start)..start]);
            let end = before + span.end;
            cursor = end.max(cursor);
            let rest = &data[end..];
            if rest.starts_with(CLOSES) {
                cursor = end + CLOSES.len();
            } else if !rest.is_empty() && CLOSES.starts_with(rest) {
                cursor = data.len();
                self.expect_close = &CLOSES[rest.len()..];
            } else if rest.is_empty()
                && start + OPENS.len() <= end
                && data[start..].starts_with(OPENS)
            {
                self.expect_close = CLOSES;
            }
            self.hear(one.clone(), now_ms, &mut cut);
            told(one);
        }

        // Hold back what may yet turn out to be a sequence rather than output.
        let carried = self.scanner.carried();
        let envelope = data.len() - opener_suffix(&data[cursor..]);
        let hold = if carried > 0 {
            let at = data.len().saturating_sub(carried).max(cursor);
            let from = widened_start(&data, at).max(cursor).min(envelope);
            self.held_sequence = Some(at - from);
            from
        } else {
            envelope
        };
        self.keep(&data[cursor..hold]);
        self.held = data[hold..].to_vec();
        if std::mem::take(&mut self.changed) {
            if let Some(block) = &self.running {
                cut(Cut::Changed(block.head.clone()), None);
            }
        }
    }

    fn keep(&mut self, bytes: &[u8]) {
        let Some(block) = self.running.as_mut() else {
            return;
        };
        if bytes.is_empty() {
            return;
        }
        if !block.head.interactive && contains(bytes, ALT_SCREEN) {
            block.head.interactive = true;
            self.changed = true;
        }
        block.output.extend_from_slice(bytes);
        if block.output.len() > MOST_OUTPUT {
            let over = block.output.len() - MOST_OUTPUT;
            block.output.drain(..over);
            block.head.truncated = true;
        }
    }

    fn hear(&mut self, told: Told, now_ms: u64, cut: &mut impl FnMut(Cut, Option<Block>)) {
        match told {
            Told::Cwd(path) => self.cwd = Some(path),
            Told::Command(line) => match self.running.as_mut() {
                // Some shells announce the line after the output mark.
                Some(block) if block.head.command.is_none() && block.output.is_empty() => {
                    block.head.command = Some(line.clone());
                    cut(Cut::Started(block.head.clone()), None);
                }
                _ => self.announced = Some(line),
            },
            Told::OutputBegan => {
                self.at_prompt = false;
                // A start with one still running: that one ended unsaid.
                self.end(None, now_ms, cut);
                self.next_id += 1;
                let block = Block {
                    head: Head {
                        id: self.next_id,
                        command: self.announced.take(),
                        cwd: self.cwd.clone(),
                        started_at: now_ms,
                        ended_at: None,
                        code: None,
                        interactive: false,
                        truncated: false,
                        bookmarked: false,
                    },
                    output: Vec::new(),
                };
                cut(Cut::Started(block.head.clone()), None);
                self.running = Some(block);
            }
            Told::CommandEnded { code } => self.end(code, now_ms, cut),
            // A prompt with a command still open: it was cut short — an
            // interrupt the shell did not report, a shell that never says D.
            Told::PromptBegan => {
                self.end(None, now_ms, cut);
                self.announced = None;
                self.integrated = true;
                self.at_prompt = true;
            }
            Told::Title(_) | Told::Clipboard(_) => {}
        }
    }

    fn end(&mut self, code: Option<i32>, now_ms: u64, cut: &mut impl FnMut(Cut, Option<Block>)) {
        let Some(mut block) = self.running.take() else {
            return;
        };
        block.head.ended_at = Some(now_ms);
        block.head.code = code;
        cut(Cut::Ended(block.head.clone()), Some(block));
    }
}

fn widened_start(chunk: &[u8], start: usize) -> usize {
    match start.checked_sub(OPENS.len()) {
        Some(before) if chunk[before..start] == *OPENS => before,
        _ => start,
    }
}

/// How long the end of `tail` is that could be the start of tmux's envelope.
fn opener_suffix(tail: &[u8]) -> usize {
    (1..=OPENS.len())
        .rev()
        .find(|&n| tail.len() >= n && OPENS.starts_with(&tail[tail.len() - n..]))
        .unwrap_or(0)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// A pane's finished blocks, oldest first, bounded in count and bytes.
#[derive(Debug, Default)]
pub struct History {
    blocks: VecDeque<Block>,
    bytes: usize,
}

/// How many finished blocks a pane keeps, and how much output in all.
pub const MOST_BLOCKS: usize = 300;
pub const MOST_BYTES: usize = 24 * 1024 * 1024;

impl History {
    pub fn push(&mut self, block: Block) {
        self.bytes += block.output.len();
        self.blocks.push_back(block);
        while self.blocks.len() > MOST_BLOCKS || (self.bytes > MOST_BYTES && self.blocks.len() > 1)
        {
            if let Some(gone) = self.blocks.pop_front() {
                self.bytes -= gone.output.len();
            }
        }
    }

    pub fn heads(&self) -> Vec<Head> {
        self.blocks.iter().map(|block| block.head.clone()).collect()
    }

    pub fn get(&self, id: u64) -> Option<&Block> {
        self.blocks.iter().find(|block| block.head.id == id)
    }

    /// Sets or takes away a block's bookmark; its head as it now is.
    pub fn bookmark(&mut self, id: u64, on: bool) -> Option<Head> {
        let block = self.blocks.iter_mut().find(|block| block.head.id == id)?;
        block.head.bookmarked = on;
        Some(block.head.clone())
    }

    /// The newest id kept, for numbering on from it.
    pub fn last_id(&self) -> Option<u64> {
        self.blocks.back().map(|block| block.head.id)
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
        self.bytes = 0;
    }
}

#[cfg(test)]
#[path = "blocks_tests.rs"]
mod tests;
