//! One member of a JSON object set or taken out where it stands, every other
//! byte of the text as it was: the person's order, indentation and line ends.
//! serde_json here keeps no order, so a parse and a print would sort the file.

use std::ops::Range;

use serde_json::{ser::PrettyFormatter, Serializer, Value};

/// `text`, a JSON object, with `value` under `key` in the object under
/// `outer`, that object made when missing.
pub fn put(text: &str, outer: &str, key: &str, value: &Value) -> Result<String, String> {
    match checked(text)?.get(outer) {
        Some(found) if !found.is_object() => return Err(format!("`{outer}` is not an object")),
        _ => {}
    }
    let root = object_at(text, root_open(text))?;
    let unit = match root.members.first() {
        Some(first) => indent_of(&text[first.lead.clone()]),
        None => Some("  ".to_owned()),
    };
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let style = Style { unit, newline };
    let mut out = text.to_owned();
    match root.last(outer) {
        Some(servers) => {
            let inner = object_at(text, servers.value.start)?;
            match inner.last(key) {
                Some(ours) => {
                    let rendered = style.beside(&text[ours.lead.clone()], value);
                    out.replace_range(ours.value.clone(), &rendered);
                }
                None => {
                    let base = line_indent(text, servers.key);
                    let (at, member) = style.member(text, &inner, &base, key, value);
                    out.replace_range(at, &member);
                }
            }
        }
        None => {
            let wrapped = Value::Object([(key.to_owned(), value.clone())].into_iter().collect());
            let (at, member) = style.member(text, &root, "", outer, &wrapped);
            out.replace_range(at, &member);
        }
    }
    Ok(out)
}

/// `text`, a JSON object, without `key` in the object under `outer`.
pub fn take(text: &str, outer: &str, key: &str) -> Result<String, String> {
    checked(text)?;
    let mut out = text.to_owned();
    loop {
        let root = object_at(&out, root_open(&out))?;
        let Some(servers) = root.last(outer) else {
            return Ok(out);
        };
        if !out[servers.value.clone()].starts_with('{') {
            return Ok(out);
        }
        let inner = object_at(&out, servers.value.start)?;
        let Some(at) = inner.members.iter().rposition(|one| one.name == key) else {
            return Ok(out);
        };
        let members = &inner.members;
        let gone = match at {
            _ if members.len() == 1 => inner.open + 1..inner.close,
            0 => members[0].lead.start..members[1].lead.start,
            _ => members[at - 1].value.end..members[at].value.end,
        };
        out.replace_range(gone, "");
    }
}

fn checked(text: &str) -> Result<serde_json::Map<String, Value>, String> {
    match serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err("it is not a JSON object".to_owned()),
        Err(err) => Err(err.to_string()),
    }
}

/// How the file writes what is added to it.
struct Style<'a> {
    /// One level of indentation; `None` for a file on one line.
    unit: Option<String>,
    newline: &'a str,
}

impl Style<'_> {
    /// `value` printed for a member whose line is indented by `indent`.
    fn render(&self, value: &Value, indent: &str) -> String {
        let Some(unit) = &self.unit else {
            return value.to_string();
        };
        let mut out = Vec::new();
        let mut printer =
            Serializer::with_formatter(&mut out, PrettyFormatter::with_indent(unit.as_bytes()));
        serde::Serialize::serialize(value, &mut printer).expect("a Value always prints");
        String::from_utf8(out)
            .expect("serde_json prints UTF-8")
            .replace('\n', &format!("{}{indent}", self.newline))
    }

    /// `value` for a member after `lead`: on a line of its own, in the file's
    /// indentation; beside others on one line, compact.
    fn beside(&self, lead: &str, value: &Value) -> String {
        match indent_of(lead) {
            Some(indent) => self.render(value, &indent),
            None => value.to_string(),
        }
    }

    /// Where a new member goes in `object`, whose line is indented by `base`,
    /// and its text.
    fn member(
        &self,
        text: &str,
        object: &Object,
        base: &str,
        key: &str,
        value: &Value,
    ) -> (Range<usize>, String) {
        let name = Value::from(key).to_string();
        match object.members.last() {
            Some(last) => {
                let lead = &text[last.lead.clone()];
                let between = &text[last.between.clone()];
                let member = format!(",{lead}{name}{between}{}", self.beside(lead, value));
                (last.value.end..last.value.end, member)
            }
            None => {
                let inside = object.open + 1..object.close;
                let member = match &self.unit {
                    Some(unit) => {
                        let indent = format!("{base}{unit}");
                        let nl = self.newline;
                        format!(
                            "{nl}{indent}{name}: {}{nl}{base}",
                            self.render(value, &indent)
                        )
                    }
                    None => format!("{name}:{}", self.render(value, "")),
                };
                (inside, member)
            }
        }
    }
}

/// The indentation after the last line break of `lead`, when it has one.
fn indent_of(lead: &str) -> Option<String> {
    lead.rfind('\n').map(|at| lead[at + 1..].to_owned())
}

/// The whitespace that starts the line `at` is on.
fn line_indent(text: &str, at: usize) -> String {
    let start = text[..at].rfind('\n').map_or(0, |found| found + 1);
    text[start..at]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

struct Member {
    name: String,
    /// The whitespace before it, after `{` or `,`.
    lead: Range<usize>,
    /// Its key, from the opening quote.
    key: usize,
    /// From the end of its key to its value: the colon and the spaces around it.
    between: Range<usize>,
    value: Range<usize>,
}

struct Object {
    open: usize,
    close: usize,
    members: Vec<Member>,
}

impl Object {
    /// The member named `name`, the last one as serde_json reads duplicates.
    fn last(&self, name: &str) -> Option<&Member> {
        self.members.iter().rev().find(|one| one.name == name)
    }
}

fn root_open(text: &str) -> usize {
    let start = text.len() - text.trim_start_matches('\u{feff}').len();
    skip_space(text.as_bytes(), start)
}

fn skip_space(bytes: &[u8], mut at: usize) -> usize {
    while bytes
        .get(at)
        .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
    {
        at += 1;
    }
    at
}

fn broken() -> String {
    "the JSON could not be followed; nothing was changed".to_owned()
}

/// Past the closing quote of the string opening at `at`.
fn string_end(bytes: &[u8], at: usize) -> Result<usize, String> {
    let mut i = at + 1;
    while let Some(b) = bytes.get(i) {
        match b {
            b'\\' => i += 2,
            b'"' => return Ok(i + 1),
            _ => i += 1,
        }
    }
    Err(broken())
}

/// Past the end of the value starting at `at`.
fn value_end(bytes: &[u8], at: usize) -> Result<usize, String> {
    match bytes.get(at) {
        Some(b'"') => string_end(bytes, at),
        Some(b'{' | b'[') => {
            let (mut depth, mut i) = (0usize, at);
            while let Some(b) = bytes.get(i) {
                match b {
                    b'"' => {
                        i = string_end(bytes, i)?;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return Ok(i + 1);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            Err(broken())
        }
        Some(_) => {
            let mut i = at;
            while bytes
                .get(i)
                .is_some_and(|b| !matches!(b, b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r'))
            {
                i += 1;
            }
            Ok(i)
        }
        None => Err(broken()),
    }
}

/// The object whose `{` is at `open`, already checked to be valid JSON.
fn object_at(text: &str, open: usize) -> Result<Object, String> {
    let bytes = text.as_bytes();
    if bytes.get(open) != Some(&b'{') {
        return Err(broken());
    }
    let mut members = Vec::new();
    let mut lead = open + 1;
    let mut at = skip_space(bytes, lead);
    if bytes.get(at) == Some(&b'}') {
        return Ok(Object {
            open,
            close: at,
            members,
        });
    }
    loop {
        if bytes.get(at) != Some(&b'"') {
            return Err(broken());
        }
        let key_end = string_end(bytes, at)?;
        let name: String = serde_json::from_str(&text[at..key_end]).map_err(|_| broken())?;
        let colon = skip_space(bytes, key_end);
        if bytes.get(colon) != Some(&b':') {
            return Err(broken());
        }
        let start = skip_space(bytes, colon + 1);
        let end = value_end(bytes, start)?;
        members.push(Member {
            name,
            lead: lead..at,
            key: at,
            between: key_end..start,
            value: start..end,
        });
        let next = skip_space(bytes, end);
        match bytes.get(next) {
            Some(b',') => {
                lead = next + 1;
                at = skip_space(bytes, lead);
            }
            Some(b'}') => {
                return Ok(Object {
                    open,
                    close: next,
                    members,
                })
            }
            _ => return Err(broken()),
        }
    }
}

#[cfg(test)]
#[path = "json_in_place_tests.rs"]
mod tests;
