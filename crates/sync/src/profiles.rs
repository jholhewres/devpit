//! The person's agent profiles on every machine, without their secrets.
//!
//! A profile is a record, `profile:<id>`, by the id minted once when it was
//! made. Its label, agent, program, arguments and models travel. Of its
//! environment every name travels, so another machine knows what to ask
//! for, and a value only for names on an allowlist: the account's config
//! folder (as `~/…`), base URLs, model names and the timeout. Every other
//! value arrives empty and is filled on each machine; a value filled here is
//! never overwritten by an empty one.
//!
//! A base URL decides where a profile's token is sent, so one changed on
//! another machine does not replace this machine's: it waits in
//! `agents.profiles.pending`, apart from the profile so an edit does not drop
//! it, until the person takes it here or keeps their own.

use std::collections::BTreeMap;
use std::path::Path;

use devpit_core::{preference as key, Store};
use serde_json::{json, Value};

use crate::space::{Here, Namespace, Seen};

pub const LABEL: &str = "label";
pub const BASE: &str = "base";
pub const COMMAND: &str = "command";
pub const ARGS: &str = "args";
pub const MODELS: &str = "models";
/// `env.<NAME>`: the value when the name is on the allowlist, else empty.
pub const ENV: &str = "env.";

/// The profiles namespace, over this machine's store and home folder.
pub struct Profiles<'a> {
    pub store: &'a Store,
    pub home: &'a Path,
}

/// A profile's record.
pub fn identity(id: &str) -> String {
    format!("profile:{id}")
}

/// Whether a variable's value travels: what names an account's folder, a
/// gateway or a model, never what proves who the person is.
pub fn shared(name: &str) -> bool {
    matches!(name, "CLAUDE_CONFIG_DIR" | "API_TIMEOUT_MS")
        || name.ends_with("_BASE_URL")
        || name.contains("_MODEL")
}

/// A variable whose change decides where the profile's token goes.
fn guarded(name: &str) -> bool {
    name.ends_with("_BASE_URL")
}

fn plain(id: &str) -> bool {
    devpit_core::home::plain_id(id)
}

fn env_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && name
            .bytes()
            .enumerate()
            .all(|(at, b)| b == b'_' || b.is_ascii_alphabetic() || (at > 0 && b.is_ascii_digit()))
}

fn clean(text: &str, longest: usize) -> bool {
    text.chars().count() <= longest && !text.chars().any(char::is_control)
}

fn list_of(value: &str) -> Option<Vec<String>> {
    let list: Vec<String> = serde_json::from_str(value).ok()?;
    (list.len() <= 64 && list.iter().all(|one| clean(one, 4096))).then_some(list)
}

/// Whether `value` may be written as `field`.
pub fn fits(field: &str, value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    match field {
        LABEL => !value.trim().is_empty() && clean(value, 120),
        BASE => plain(value),
        COMMAND => {
            value.is_empty()
                || (clean(value, 128)
                    && !value.contains(['/', '\\'])
                    && !value.chars().any(char::is_whitespace))
        }
        ARGS | MODELS => list_of(value).is_some(),
        field => match field.strip_prefix(ENV) {
            Some(name) if env_name(name) && shared(name) => clean(value, 4096),
            Some(name) if env_name(name) => value.is_empty(),
            _ => false,
        },
    }
}

impl Profiles<'_> {
    fn home(&self) -> String {
        self.home.to_string_lossy().trim_end_matches('/').to_owned()
    }

    /// A value as another machine may read it: under the home folder as `~`.
    fn portable(&self, value: &str) -> Option<String> {
        let home = self.home();
        if value == home {
            return Some("~".to_owned());
        }
        if let Some(rest) = value.strip_prefix(&format!("{home}/")) {
            return Some(format!("~/{rest}"));
        }
        // A path elsewhere on this machine means nothing on another.
        (!value.starts_with('/')).then(|| value.to_owned())
    }

    /// A value from another machine, under this one's home.
    fn local(&self, value: &str) -> String {
        match value.strip_prefix("~/") {
            Some(rest) => format!("{}/{rest}", self.home()),
            None if value == "~" => self.home(),
            None => value.to_owned(),
        }
    }

    /// The stored list, or `None` when it cannot be read: writing over a
    /// list nobody can parse would drop every profile on it.
    fn list(&self) -> Result<Option<Vec<Value>>, String> {
        let raw = self
            .store
            .preference(key::AGENT_PROFILES)
            .map_err(|err| err.to_string())?
            .unwrap_or_default();
        if raw.trim().is_empty() {
            return Ok(Some(Vec::new()));
        }
        Ok(serde_json::from_str::<Vec<Value>>(&raw).ok())
    }

    /// What waits for the person, by profile id and name.
    pub fn pending(&self) -> Result<BTreeMap<String, BTreeMap<String, String>>, String> {
        let raw = self
            .store
            .preference(key::AGENT_PROFILES_PENDING)
            .map_err(|err| err.to_string())?
            .unwrap_or_default();
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    fn keep_pending(&self, all: &BTreeMap<String, BTreeMap<String, String>>) -> Result<(), String> {
        self.store
            .set_preference(key::AGENT_PROFILES_PENDING, &json!(all).to_string())
            .map_err(|err| err.to_string())
    }

    fn keep(&self, list: &[Value]) -> Result<(), String> {
        self.store
            .set_preference(key::AGENT_PROFILES, &Value::from(list.to_vec()).to_string())
            .map_err(|err| err.to_string())
    }
}

fn text(profile: &Value, name: &str) -> String {
    profile[name].as_str().unwrap_or_default().to_owned()
}

fn vars(profile: &Value, name: &str) -> Vec<(String, String)> {
    profile[name]
        .as_array()
        .map(|all| {
            all.iter()
                .filter_map(|var| Some((var["name"].as_str()?.to_owned(), text(var, "value"))))
                .collect()
        })
        .unwrap_or_default()
}

fn set_var(profile: &mut Value, list: &str, name: &str, value: Option<&str>) {
    let mut kept: Vec<Value> = profile[list].as_array().cloned().unwrap_or_default();
    let at = kept.iter().position(|var| var["name"] == name);
    match (at, value) {
        (Some(at), Some(value)) => kept[at]["value"] = json!(value),
        (None, Some(value)) => kept.push(json!({"name": name, "value": value})),
        (Some(at), None) => {
            kept.remove(at);
        }
        (None, None) => {}
    }
    profile[list] = Value::Array(kept);
}

impl Namespace for Profiles<'_> {
    fn name(&self) -> &'static str {
        "profiles"
    }

    fn here(&self) -> Result<Here, String> {
        let mut here = Here::new();
        let waiting = self.pending()?;
        for profile in self.list()?.unwrap_or_default() {
            let id = text(&profile, "id");
            // Not a profile anyone could start: nothing to carry.
            if !plain(&id)
                || !fits(LABEL, Some(&text(&profile, LABEL)))
                || !fits(BASE, Some(&text(&profile, BASE)))
            {
                continue;
            }
            let mut fields = BTreeMap::new();
            let mut put = |field: String, value: String| {
                if fits(&field, Some(&value)) {
                    fields.insert(field, Seen::of(Some(value), None));
                }
            };
            put(LABEL.to_owned(), text(&profile, LABEL));
            put(BASE.to_owned(), text(&profile, BASE));
            let command = text(&profile, COMMAND);
            put(
                COMMAND.to_owned(),
                if fits(COMMAND, Some(&command)) {
                    command
                } else {
                    String::new()
                },
            );
            put(
                ARGS.to_owned(),
                profile[ARGS]
                    .as_array()
                    .map(|all| Value::from(all.clone()).to_string())
                    .unwrap_or_else(|| "[]".to_owned()),
            );
            put(
                MODELS.to_owned(),
                profile[MODELS]
                    .as_array()
                    .map(|all| Value::from(all.clone()).to_string())
                    .unwrap_or_else(|| "[]".to_owned()),
            );
            let none = BTreeMap::new();
            let pending = waiting.get(&id).unwrap_or(&none);
            for (name, value) in vars(&profile, "env") {
                if !shared(&name) {
                    put(format!("{ENV}{name}"), String::new());
                    continue;
                }
                // What waits for the person here is what settled.
                let value = pending.get(&name).unwrap_or(&value);
                if let Some(portable) = self.portable(value) {
                    put(format!("{ENV}{name}"), portable);
                }
            }
            here.insert(identity(&id), fields);
        }
        Ok(here)
    }

    fn write(&self, record: &str, field: &str, value: Option<&str>) -> Result<(), String> {
        let (Some(id), Some(value)) = (record.strip_prefix("profile:"), value) else {
            return Ok(());
        };
        if !plain(id) || !fits(field, Some(value)) {
            return Ok(());
        }
        let Some(mut list) = self.list()? else {
            return Ok(());
        };
        let at = match list.iter().position(|one| one["id"] == id) {
            Some(at) => at,
            None => {
                list.push(json!({
                    "id": id, "label": "", "base": "", "command": "",
                    "args": [], "env": [], "models": []
                }));
                list.len() - 1
            }
        };
        let profile = &mut list[at];
        match field {
            LABEL | BASE | COMMAND => profile[field] = json!(value),
            ARGS | MODELS => profile[field] = json!(list_of(value).unwrap_or_default()),
            field => {
                let Some(name) = field.strip_prefix(ENV) else {
                    return Ok(());
                };
                let now = vars(profile, "env")
                    .into_iter()
                    .find(|(one, _)| one == name)
                    .map(|(_, value)| value);
                if !shared(name) {
                    // A secret: named here so it can be asked for, never
                    // emptied once this machine has it.
                    if now.is_none() {
                        set_var(profile, "env", name, Some(""));
                    }
                } else {
                    let value = self.local(value);
                    let mut waiting = self.pending()?;
                    let mine = waiting.entry(id.to_owned()).or_default();
                    match now {
                        Some(now) if guarded(name) && !now.is_empty() && now != value => {
                            mine.insert(name.to_owned(), value);
                        }
                        _ => {
                            set_var(profile, "env", name, Some(&value));
                            mine.remove(name);
                        }
                    }
                    waiting.retain(|_, names| !names.is_empty());
                    if waiting != self.pending()? {
                        self.keep_pending(&waiting)?;
                    }
                }
            }
        }
        self.keep(&list)
    }

    fn welcomes(&self, record: &str) -> bool {
        record.strip_prefix("profile:").is_some_and(plain)
    }
}

#[cfg(test)]
#[path = "profiles_tests.rs"]
mod tests;
