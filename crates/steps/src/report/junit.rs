//! JUnit XML: pytest, Node's runner, go-junit-report, Maven and Gradle.

use std::path::Path;

use devpit_rpc::{FailedTest, Tested};
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

/// One `<testcase>` while it is being read.
struct Case {
    name: String,
    file: Option<String>,
    failed: bool,
    skipped: bool,
    message: Option<String>,
}

pub(super) fn from_xml(text: &str, cwd: &Path) -> Option<Tested> {
    let mut reader = Reader::from_str(text);
    let mut tested: Option<Tested> = None;
    let mut case: Option<Case> = None;
    let mut in_failure = false;

    loop {
        let event = reader.read_event().ok()?;
        match &event {
            Event::Start(tag) | Event::Empty(tag) => {
                let empty = matches!(event, Event::Empty(_));
                match tag.name().as_ref() {
                    b"testsuites" | b"testsuite" => {
                        tested.get_or_insert_with(|| Tested {
                            read_from: vec!["JUnit".to_owned()],
                            ..Tested::default()
                        });
                    }
                    b"testcase" => {
                        let started = Case {
                            name: attribute(tag, "name").unwrap_or_default(),
                            file: attribute(tag, "file").map(|file| super::relative(&file, cwd)),
                            failed: false,
                            skipped: false,
                            message: None,
                        };
                        if empty {
                            ended(tested.as_mut()?, started);
                        } else {
                            case = Some(started);
                        }
                    }
                    b"failure" | b"error" => {
                        if let Some(case) = case.as_mut() {
                            case.failed = true;
                            case.message = attribute(tag, "message").and_then(|m| super::short(&m));
                            in_failure = !empty;
                        }
                    }
                    b"skipped" => {
                        if let Some(case) = case.as_mut() {
                            case.skipped = true;
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(said) if in_failure => {
                if let Some(case) = case.as_mut().filter(|case| case.message.is_none()) {
                    case.message = said.decode().ok().and_then(|text| super::short(&text));
                }
            }
            Event::End(tag) => match tag.name().as_ref() {
                b"failure" | b"error" => in_failure = false,
                b"testcase" => {
                    if let Some(done) = case.take() {
                        ended(tested.as_mut()?, done);
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    tested
}

/// Counts a finished case.
fn ended(tested: &mut Tested, case: Case) {
    if case.failed {
        tested.failed += 1;
        if tested.failures.len() < devpit_rpc::MOST_FAILURES_KEPT {
            tested.failures.push(FailedTest {
                name: case.name,
                file: case.file,
                message: case.message,
            });
        }
    } else if case.skipped {
        tested.skipped += 1;
    } else {
        tested.passed += 1;
    }
}

fn attribute(tag: &BytesStart, name: &str) -> Option<String> {
    let found = tag.try_get_attribute(name).ok()??;
    Some(
        found
            .normalized_value(XmlVersion::Implicit1_0)
            .ok()?
            .into_owned(),
    )
}
