//! A project agent's memory: `.devpit/project-agent/` in the project's folder.
//!
//! Three Markdown files of dated entries, each naming where it came from, so
//! a fact can be checked against its source and dropped when it goes stale.
//! Written once, on the agent's first wake, from the guides the project
//! already keeps; never written over after that. A project whose agent was
//! never asked has no such folder, and nothing else reads one.

use std::io::Read;
use std::path::Path;

pub(crate) const DIR: &str = ".devpit/project-agent";
pub(crate) const FILES: [&str; 3] = ["facts.md", "lessons.md", "decisions.md"];
/// The entries of each file the agent wakes with; it reads the rest itself.
pub(crate) const SHOWN: usize = 20;
const SEP: &str = " · ";
/// How much of a guide or an ADR is read for its heading.
pub(crate) const HEAD_BYTES: usize = 16 * 1024;
/// How much of a memory file the summary reads.
const MEMORY_BYTES: u64 = 256 * 1024;

/// The guides a first wake points at, when the project has them.
const GUIDES: [&str; 4] = [
    "AGENTS.md",
    "CLAUDE.md",
    "README.md",
    "docs/architecture.md",
];

pub(crate) struct Entry {
    pub day: String,
    pub source: String,
    pub text: String,
}

/// A bullet as an entry, or why it is not one; `None` for any other line.
pub(crate) fn entry(line: &str) -> Option<Result<Entry, String>> {
    let rest = line.strip_prefix("- ")?;
    let mut parts = rest.splitn(3, SEP);
    let (Some(day), Some(source), Some(text)) = (parts.next(), parts.next(), parts.next()) else {
        return Some(Err(format!(
            "an entry is `- YYYY-MM-DD{SEP}source{SEP}what it says`"
        )));
    };
    if !dated(day) {
        return Some(Err(format!("`{day}` is not a date as YYYY-MM-DD")));
    }
    let (source, text) = (source.trim(), text.trim());
    if source.is_empty() || text.is_empty() {
        return Some(Err(
            "an entry names its source and says something".to_owned()
        ));
    }
    Some(Ok(Entry {
        day: day.to_owned(),
        source: source.to_owned(),
        text: text.to_owned(),
    }))
}

fn dated(day: &str) -> bool {
    let bytes = day.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(at, byte)| match at {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

/// Writes the memory on the agent's first wake: false when it is already kept.
pub(crate) fn seed(root: &Path, day: &str) -> Result<bool, String> {
    let dir = root.join(DIR);
    if dir.exists() {
        return Ok(false);
    }
    // Resolved inside the project, a link included, before anything is written.
    for folder in [".devpit", DIR] {
        let at = devpit_core::paths::resolve_new(root, folder).map_err(|err| err.to_string())?;
        std::fs::create_dir_all(&at).map_err(|err| err.to_string())?;
    }
    let facts = GUIDES.iter().filter_map(|guide| {
        let head = read_inside(root, guide, HEAD_BYTES as u64)?;
        let title = heading(&head, guide);
        Some(line(
            day,
            guide,
            &format!("A guide to read first: \"{title}\"."),
        ))
    });
    let decisions = adrs(root).into_iter().filter_map(|adr| {
        let head = read_inside(root, &adr, HEAD_BYTES as u64)?;
        Some(line(day, &adr, &heading(&head, &adr)))
    });
    let files = [
        ("facts.md", "Facts", facts.collect::<Vec<_>>()),
        ("lessons.md", "Lessons", Vec::new()),
        ("decisions.md", "Decisions", decisions.collect()),
    ];
    for (file, title, lines) in files {
        let at = devpit_core::paths::resolve_new(root, &format!("{DIR}/{file}"))
            .map_err(|err| err.to_string())?;
        let mut text = format!(
            "# {title}\n\nEach entry: `- YYYY-MM-DD{SEP}source{SEP}what it says`. The source is a file, a card (#12) or a PR (PR #3).\n\n"
        );
        for line in lines {
            text.push_str(&line);
        }
        std::fs::write(at, text).map_err(|err| err.to_string())?;
    }
    Ok(true)
}

fn line(day: &str, source: &str, text: &str) -> String {
    format!(
        "- {day}{SEP}{source}{SEP}{}\n",
        text.replace(['\n', '\r'], " ")
    )
}

/// A regular file of the project, resolved inside its root and read up to
/// `limit` bytes. `None` for a link leading out, a device or a folder.
fn read_inside(root: &Path, relative: &str, limit: u64) -> Option<String> {
    let at = devpit_core::tree::resolve(root, relative).ok()?;
    if !at.metadata().ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    std::fs::File::open(at)
        .ok()?
        .take(limit)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// The first heading of `text`, or the file's own name when it has none.
fn heading(text: &str, file: &str) -> String {
    text.lines()
        .find_map(|line| line.strip_prefix("# ").map(|title| title.trim().to_owned()))
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| file.to_owned())
}

fn adrs(root: &Path) -> Vec<String> {
    let Ok(folder) = devpit_core::tree::resolve(root, "docs/adr") else {
        return Vec::new();
    };
    let mut found: Vec<String> = std::fs::read_dir(folder)
        .map(|dir| {
            dir.filter_map(Result::ok)
                .filter_map(|one| one.file_name().into_string().ok())
                .filter(|name| name.ends_with(".md"))
                .map(|name| format!("docs/adr/{name}"))
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

/// What the agent wakes with: each file's newest entries and what was left
/// out. `None` while the project has no memory.
pub(crate) fn summary(root: &Path) -> Option<String> {
    let dir = root.join(DIR);
    if !dir.is_dir() {
        return None;
    }
    let mut out = format!(
        "Your memory is kept in {DIR}/: these are its newest entries; read the files for the rest.\n"
    );
    for file in FILES {
        let text = read_inside(root, &format!("{DIR}/{file}"), MEMORY_BYTES).unwrap_or_default();
        let (mut kept, mut broken) = (Vec::new(), 0);
        for found in text.lines().filter_map(entry) {
            match found {
                Ok(one) => kept.push(one),
                Err(_) => broken += 1,
            }
        }
        kept.sort_by(|a, b| b.day.cmp(&a.day));
        out.push_str(&format!("\n{file} ({} entries)\n", kept.len()));
        for one in kept.iter().take(SHOWN) {
            out.push_str(&format!(
                "- {}{SEP}{}{SEP}{}\n",
                one.day, one.source, one.text
            ));
        }
        if kept.len() > SHOWN {
            out.push_str(&format!("({} older in the file)\n", kept.len() - SHOWN));
        }
        if broken > 0 {
            let lines = if broken == 1 { "line" } else { "lines" };
            out.push_str(&format!(
                "({broken} {lines} left out: an entry needs its date and its source)\n"
            ));
        }
    }
    Some(out)
}

#[cfg(test)]
#[path = "project_memory_tests.rs"]
mod tests;
