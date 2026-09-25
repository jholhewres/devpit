//! A question a session is stopped on, read off its screen.
//!
//! The screen and not the hooks: a hook says a session is waiting the moment
//! it starts to, and nothing after — which question, which choices, which one
//! is under the cursor — while the screen says all of it for as long as it is
//! true. Claude Code draws its choices one per line, numbered from 1, the one
//! under the cursor marked `❯`; a numbered list in ordinary output has no
//! cursor, which is how the two are told apart.

use devpit_rpc::{PendingPrompt, PromptOption};

/// How far up the screen a question is looked for: the bottom of a pane is
/// where a prompt is drawn, and a long history is not read.
const LOOKED_AT: usize = 60;

const CURSOR: char = '❯';

/// The question on this screen, if it is stopped on one.
pub(crate) fn pending(screen: &str) -> Option<PendingPrompt> {
    let lines: Vec<&str> = screen.lines().collect();
    let lines = &lines[lines.len().saturating_sub(LOOKED_AT)..];

    // Every numbered line, with where it is and whether the cursor is on it.
    let numbered: Vec<(usize, u32, bool, &str)> = lines
        .iter()
        .enumerate()
        .filter_map(|(at, line)| option_line(line).map(|(n, marked, label)| (at, n, marked, label)))
        .collect();

    // The last run numbered 1, 2, 3… that has the cursor on one of its lines.
    let mut runs: Vec<Vec<(usize, u32, bool, &str)>> = Vec::new();
    for one in numbered {
        let follows = runs.last().and_then(|run| run.last()).is_some_and(|last| {
            last.1 + 1 == one.1
                && lines[last.0 + 1..one.0]
                    .iter()
                    .all(|between| between_options(between, lines[last.0]))
        });
        if one.1 == 1 {
            runs.push(vec![one]);
        } else if follows {
            runs.last_mut().expect("a run to follow").push(one);
        }
    }
    let run = runs
        .into_iter()
        .rev()
        .find(|run| run.len() >= 2 && run.iter().any(|one| one.2))?;

    // Claude Code's own input box is a `❯` between two rules: a list typed
    // there is the person's message, not a question.
    let above = lines[..run[0].0]
        .iter()
        .rev()
        .find(|line| !line.trim().is_empty());
    if above.is_some_and(|line| rule(line.trim())) {
        return None;
    }
    let question = question_above(lines, run[0].0);
    if question.is_empty() {
        return None;
    }

    let options = run
        .iter()
        .map(|&(at, _, _, label)| PromptOption {
            label: label.trim().to_owned(),
            hint: hint_below(lines, at),
        })
        .collect();
    let cursor = run.iter().position(|one| one.2)? as u32;
    Some(PendingPrompt {
        question,
        options,
        cursor,
    })
}

/// What may stand between two choices: the first one's description, indented
/// under it, or the rule Claude Code draws before its last choice.
fn between_options(line: &str, option: &str) -> bool {
    let said = line.trim();
    rule(said) || (!said.is_empty() && indent(line) > indent(option))
}

/// `❯ 2. Label` or `  2. Label`: the number, whether the cursor is on it, and
/// the label.
fn option_line(line: &str) -> Option<(u32, bool, &str)> {
    let rest = line.trim_start();
    let (marked, rest) = match rest.strip_prefix(CURSOR) {
        Some(after) => (true, after.trim_start()),
        None => (false, rest),
    };
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 2 {
        return None;
    }
    let number: u32 = rest[..digits].parse().ok()?;
    let label = rest[digits..].strip_prefix(". ")?;
    (!label.trim().is_empty()).then_some((number, marked, label))
}

/// The line under an option, when it is its description: indented, not the
/// next option, not a rule.
fn hint_below(lines: &[&str], at: usize) -> Option<String> {
    let next = lines.get(at + 1)?;
    let said = next.trim();
    (!said.is_empty()
        && option_line(next).is_none()
        && !rule(said)
        && indent(next) > indent(lines[at]))
    .then(|| said.to_owned())
}

/// How many lines of words a question is read from, at most.
const QUESTION_LINES: usize = 8;

/// The words above the choices, up to a rule or the tab row: blank lines part
/// its paragraphs rather than end it, since a permission prompt says what it
/// asks about — the command — a paragraph above "Do you want to proceed?".
fn question_above(lines: &[&str], first: usize) -> String {
    let mut paragraphs: Vec<Vec<&str>> = vec![Vec::new()];
    let mut read = 0;
    for line in lines[..first].iter().rev() {
        let line = line.trim();
        if rule(line) || tabs(line) || said_before(line) || read == QUESTION_LINES {
            break;
        }
        if line.is_empty() {
            if !paragraphs.last().is_some_and(Vec::is_empty) {
                paragraphs.push(Vec::new());
            }
            continue;
        }
        paragraphs
            .last_mut()
            .expect("a paragraph")
            .push(line.trim_start_matches('│').trim());
        read += 1;
    }
    paragraphs
        .into_iter()
        .rev()
        .filter(|words| !words.is_empty())
        .map(|mut words| {
            words.reverse();
            words.join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rule(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|ch| matches!(ch, '─' | '━' | '-' | '═' | ' '))
}

/// A line of the conversation above a question rather than the question:
/// another list of choices, or a turn — the person's (`>`), Claude's (`●`) or
/// a tool's output (`⎿`).
fn said_before(line: &str) -> bool {
    option_line(line).is_some() || line.starts_with(['>', '●', '⎿'])
}

/// Claude Code's row of question tabs: `← □ One □ Two ✔ Submit →`.
fn tabs(line: &str) -> bool {
    line.starts_with('←') || line.contains('□') || line.contains('✔')
}

fn indent(line: &str) -> usize {
    line.chars().take_while(|ch| ch.is_whitespace()).count()
}

#[cfg(test)]
#[path = "live_prompt_tests.rs"]
mod tests;
