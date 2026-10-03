//! What `cargo test` printed: libtest's JSON lines on nightly, its
//! `test result:` lines everywhere else.

use devpit_rpc::{FailedTest, Tested};

/// The output's tests, or `None` when no suite in it finished.
pub(super) fn from_output(output: &str) -> Option<Tested> {
    from_json_lines(output).or_else(|| from_text(output))
}

/// `-Z unstable-options --format json`: a `suite` line ends each binary.
fn from_json_lines(output: &str) -> Option<Tested> {
    let mut tested: Option<Tested> = None;
    let mut failures = Vec::new();
    for line in output
        .lines()
        .filter(|line| line.trim_start().starts_with('{'))
    {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let kind = event["type"].as_str();
        let what = event["event"].as_str();
        match (kind, what) {
            (Some("suite"), Some("ok" | "failed")) => {
                let count = |key: &str| event[key].as_u64().unwrap_or(0) as u32;
                let suite = tested.get_or_insert_with(|| Tested {
                    read_from: vec!["cargo test".to_owned()],
                    ..Tested::default()
                });
                suite.passed += count("passed");
                suite.failed += count("failed");
                suite.skipped += count("ignored");
            }
            (Some("test"), Some("failed")) => {
                let said = event["stdout"].as_str().unwrap_or_default();
                let (file, message) = panic_of(said);
                failures.push(FailedTest {
                    name: event["name"].as_str().unwrap_or_default().to_owned(),
                    file,
                    message,
                });
            }
            _ => {}
        }
    }
    let mut tested = tested?;
    tested.failures = failures;
    tested.failures.truncate(devpit_rpc::MOST_FAILURES_KEPT);
    Some(tested)
}

/// `test result: FAILED. 1 passed; 1 failed; 1 ignored; …`, one per binary,
/// and `test name ... FAILED` for each that failed.
fn from_text(output: &str) -> Option<Tested> {
    let mut tested: Option<Tested> = None;
    let mut names = Vec::new();
    for line in output.lines() {
        if let Some(counts) = line.trim().strip_prefix("test result: ") {
            let suite = tested.get_or_insert_with(|| Tested {
                read_from: vec!["cargo test".to_owned()],
                ..Tested::default()
            });
            suite.passed += counted(counts, "passed");
            suite.failed += counted(counts, "failed");
            suite.skipped += counted(counts, "ignored");
        } else if let Some(name) = line
            .strip_prefix("test ")
            .and_then(|rest| rest.strip_suffix(" ... FAILED"))
        {
            names.push(name.to_owned());
        }
    }
    let mut tested = tested?;
    tested.failures = names
        .into_iter()
        .take(devpit_rpc::MOST_FAILURES_KEPT)
        .map(|name| {
            let (file, message) = panic_of(said_by(output, &name));
            FailedTest {
                name,
                file,
                message,
            }
        })
        .collect();
    Some(tested)
}

/// The number before `word` in `1 passed; 1 failed; …`.
fn counted(counts: &str, word: &str) -> u32 {
    counts
        .split(';')
        .find_map(|part| {
            let mut words = part.split_whitespace().rev();
            (words.next()? == word).then(|| words.next()?.parse().ok())?
        })
        .unwrap_or(0)
}

/// What a failed test printed: its `---- name stdout ----` block.
fn said_by<'a>(output: &'a str, name: &str) -> &'a str {
    let header = format!("---- {name} stdout ----");
    let Some(start) = output.find(&header) else {
        return "";
    };
    let rest = &output[start + header.len()..];
    let end = rest.find("\n---- ").or_else(|| rest.find("\nfailures:"));
    &rest[..end.unwrap_or(rest.len())]
}

/// Where a test panicked and what it said: `panicked at src/lib.rs:6:40:`
/// and the line after it.
fn panic_of(said: &str) -> (Option<String>, Option<String>) {
    let mut lines = said.lines();
    let Some(at) = lines.find_map(|line| line.split_once("panicked at ").map(|(_, at)| at)) else {
        return (None, super::short(said));
    };
    let file = at.split(':').next().filter(|file| !file.is_empty());
    let message = at
        .splitn(4, ':')
        .nth(3)
        .and_then(super::short)
        .or_else(|| lines.next().and_then(super::short));
    (file.map(str::to_owned), message)
}
