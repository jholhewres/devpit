//! An orchestrator on every machine the person uses: its name, the account
//! it speaks as, and the projects linked to it.
//!
//! Its record is `orch:<sync id>`, the id minted once from its folder's name
//! and kept in its settings: the same orchestrator set up on another machine
//! under the same name is the same record. A link travels as the repository
//! it names, `link.<origin hash>`, so a project's local id and its folder
//! never leave. A link to a project without an origin stays on its machine;
//! one to a repository this machine does not have is kept as `elsewhere`,
//! and taken up when the repository is cloned here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use devpit_core::Store;
use serde_json::{json, Value};

use crate::projects::carriers;
use crate::space::{Here, Namespace, Seen};

pub const NAME: &str = "name";
pub const PROFILE: &str = "profile";
/// `link.<origin hash>`: `on` or `off`.
pub const LINK: &str = "link.";

const SYNC_ID: &str = "sync_id";
const LINKED: &str = "projects";
const ELSEWHERE: &str = "elsewhere";

/// The orchestrators namespace, over this machine's store and home.
pub struct Orchestrators<'a> {
    pub store: &'a Store,
    pub root: &'a Path,
}

/// An orchestrator on this machine.
struct Local {
    id: String,
    name: String,
    folder: PathBuf,
    slug: String,
}

/// The record of the orchestrator whose folder is called `slug`.
pub fn identity(sync_id: &str) -> String {
    format!("orch:{sync_id}")
}

/// The id an orchestrator is minted, from its folder's name: the same on
/// every machine that sets it up under that name.
pub fn minted(slug: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(format!("devpit orchestrator/{slug}").as_bytes())
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn plain(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

fn hash_of(field: &str) -> Option<&str> {
    field
        .strip_prefix(LINK)
        .filter(|hash| hash.len() == 32 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Whether `value` may be written as `field`.
pub fn fits(field: &str, value: Option<&str>) -> bool {
    match (field, value) {
        (NAME, Some(name)) => !name.trim().is_empty() && name.chars().count() <= 120,
        (PROFILE, Some(profile)) => plain(profile),
        (field, Some(state)) => hash_of(field).is_some() && matches!(state, "on" | "off"),
        _ => false,
    }
}

fn strings(settings: &Value, name: &str) -> Vec<String> {
    settings[name]
        .as_array()
        .map(|all| {
            all.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

impl Orchestrators<'_> {
    /// The orchestrators on this machine: projects in devpit's own folder.
    fn locals(&self) -> Result<Vec<Local>, String> {
        let under = [
            self.root.join("orchestrator"),
            self.root
                .canonicalize()
                .unwrap_or_else(|_| self.root.to_path_buf())
                .join("orchestrator"),
        ];
        let rows = self.store.projects().map_err(|err| err.to_string())?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let folder = PathBuf::from(&row.root_path);
                let rest = under.iter().find_map(|dir| folder.strip_prefix(dir).ok())?;
                let slug = rest.file_name()?.to_str()?.to_owned();
                plain(&slug).then_some(Local {
                    id: row.id,
                    name: row.name,
                    folder,
                    slug,
                })
            })
            .collect())
    }

    fn settings(&self, local: &Local) -> Result<Value, String> {
        self.store
            .orchestrator_settings(&local.folder)
            .map_err(|err| err.to_string())
    }

    fn set(&self, local: &Local, name: &str, value: Value) -> Result<(), String> {
        self.store
            .set_orchestrator_setting(&local.folder, name, value)
            .map_err(|err| err.to_string())
    }

    /// Its sync id, minted and kept the first time it is asked.
    fn sync_id(&self, local: &Local) -> Result<String, String> {
        let settings = self.settings(local)?;
        if let Some(kept) = settings[SYNC_ID].as_str().filter(|id| plain(id)) {
            return Ok(kept.to_owned());
        }
        let id = minted(&local.slug);
        self.set(local, SYNC_ID, json!(id))?;
        Ok(id)
    }

    /// The repositories this machine has, by origin hash, and which project
    /// carries each: an orchestrator is never one.
    fn repositories(&self) -> Result<BTreeMap<String, String>, String> {
        let projects = self
            .store
            .synced_projects()
            .map_err(|err| err.to_string())?;
        Ok(carriers(&projects)
            .into_iter()
            .filter_map(|(record, project)| {
                Some((
                    record.strip_prefix("origin:")?.to_owned(),
                    project.id.clone(),
                ))
            })
            .collect())
    }

    /// Links kept as `elsewhere` whose repository is now here become links
    /// to the project that carries it.
    fn take_up(&self, local: &Local, repos: &BTreeMap<String, String>) -> Result<(), String> {
        let settings = self.settings(local)?;
        let elsewhere = strings(&settings, ELSEWHERE);
        if !elsewhere.iter().any(|hash| repos.contains_key(hash)) {
            return Ok(());
        }
        let mut linked = strings(&settings, LINKED);
        let mut left = Vec::new();
        for hash in elsewhere {
            match repos.get(&hash) {
                Some(id) if !linked.contains(id) => linked.push(id.clone()),
                Some(_) => {}
                None => left.push(hash),
            }
        }
        self.set(local, LINKED, json!(linked))?;
        self.set(local, ELSEWHERE, json!(left))
    }

    fn by_record(&self, record: &str) -> Result<Option<Local>, String> {
        for local in self.locals()? {
            if identity(&self.sync_id(&local)?) == record {
                return Ok(Some(local));
            }
        }
        Ok(None)
    }

    fn link(&self, local: &Local, hash: &str, on: bool) -> Result<(), String> {
        let repos = self.repositories()?;
        let settings = self.settings(local)?;
        let mut linked = strings(&settings, LINKED);
        let mut elsewhere = strings(&settings, ELSEWHERE);
        elsewhere.retain(|one| one != hash);
        match (repos.get(hash), on) {
            (Some(id), true) if !linked.contains(id) => linked.push(id.clone()),
            (Some(id), false) => linked.retain(|one| one != id),
            (None, true) => elsewhere.push(hash.to_owned()),
            _ => {}
        }
        self.set(local, LINKED, json!(linked))?;
        self.set(local, ELSEWHERE, json!(elsewhere))
    }
}

impl Namespace for Orchestrators<'_> {
    fn name(&self) -> &'static str {
        "orchestrators"
    }

    fn here(&self) -> Result<Here, String> {
        let repos = self.repositories()?;
        let mut here = Here::new();
        for local in self.locals()? {
            let record = identity(&self.sync_id(&local)?);
            self.take_up(&local, &repos)?;
            let settings = self.settings(&local)?;
            let at = self
                .store
                .orchestrator_settings_at(&local.folder)
                .map_err(|err| err.to_string())?
                .and_then(|seconds| u64::try_from(seconds).ok())
                .map(|seconds| seconds * 1000);
            let linked: BTreeSet<String> = strings(&settings, LINKED).into_iter().collect();
            let mut fields = BTreeMap::new();
            let mut put = |field: String, value: Option<String>, at: Option<u64>| {
                if fits(&field, value.as_deref()) {
                    fields.insert(field, Seen::of(value, at));
                }
            };
            put(NAME.to_owned(), Some(local.name.clone()), None);
            put(
                PROFILE.to_owned(),
                settings[PROFILE].as_str().map(str::to_owned),
                at,
            );
            for (hash, id) in &repos {
                let state = if linked.contains(id) { "on" } else { "off" };
                put(format!("{LINK}{hash}"), Some(state.to_owned()), at);
            }
            for hash in strings(&settings, ELSEWHERE) {
                put(format!("{LINK}{hash}"), Some("on".to_owned()), at);
            }
            here.insert(record, fields);
        }
        Ok(here)
    }

    fn write(&self, record: &str, field: &str, value: Option<&str>) -> Result<(), String> {
        if !fits(field, value) {
            return Ok(());
        }
        let Some(local) = self.by_record(record)? else {
            return Ok(());
        };
        let value = value.unwrap_or_default();
        match field {
            NAME => {
                let Some(row) = self
                    .store
                    .project(&local.id)
                    .map_err(|err| err.to_string())?
                else {
                    return Ok(());
                };
                self.store
                    .edit_project(
                        &local.id,
                        value.trim(),
                        row.group.as_deref(),
                        row.icon.as_deref(),
                        row.color.as_deref(),
                    )
                    .map(|_| ())
                    .map_err(|err| err.to_string())
            }
            PROFILE => self.set(&local, PROFILE, json!(value)),
            link => match hash_of(link) {
                Some(hash) => self.link(&local, hash, value == "on"),
                None => Ok(()),
            },
        }
    }
}

#[cfg(test)]
#[path = "orchestrators_tests.rs"]
mod tests;
