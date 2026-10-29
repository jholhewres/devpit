//! An orchestrator on every machine the person uses: its name and the
//! projects linked to it.
//!
//! Its record is `orch:<sync id>`, the id minted once from its folder's path
//! under `orchestrator/` and kept in its settings: the same orchestrator set
//! up on another machine under the same name is the same record. A link
//! travels as the repository it names, `link.<origin hash>`, so a project's
//! local id and its folder never leave.
//!
//! Only what the person did travels: a link (`on`), and an unlink kept as a
//! tombstone (`off`). A repository nobody linked sends nothing, so a machine
//! that joins keeps its own links beside the others'. A link to a folder
//! without an origin stays on its machine; one to a repository this machine
//! does not have waits as `elsewhere` and is taken up when it is cloned. The
//! account an orchestrator speaks as is a profile of this machine's and stays
//! here until profiles themselves cross machines.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use devpit_core::home::plain_id;
use devpit_core::Store;
use serde_json::{json, Value};

use crate::space::{Here, Namespace, Seen};

pub const NAME: &str = "name";
/// `link.<origin hash>`: `on` or `off`.
pub const LINK: &str = "link.";

const SYNC_ID: &str = "sync_id";
const LINKED: &str = "projects";
const ELSEWHERE: &str = "elsewhere";
/// The repositories this machine last knew as linked, and those unlinked.
const LINKS_ON: &str = "links_on";
const LINKS_OFF: &str = "links_off";

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
    /// Its folder under `orchestrator/`: `work`, or `<profile>/work`.
    path: String,
}

/// The record of the orchestrator with this sync id.
pub fn identity(sync_id: &str) -> String {
    format!("orch:{sync_id}")
}

/// The id an orchestrator is minted from its folder's path under
/// `orchestrator/`: the same on every machine that sets it up there.
pub fn minted(path: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(format!("devpit orchestrator/{path}").as_bytes())
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
        (field, Some(state)) => hash_of(field).is_some() && matches!(state, "on" | "off"),
        _ => false,
    }
}

fn strings(settings: &Value, name: &str) -> BTreeSet<String> {
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

/// Links as the person set them, kept in the order they were made.
fn linked_of(settings: &Value) -> Vec<String> {
    settings[LINKED]
        .as_array()
        .map(|all| {
            all.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// What a round needs of this machine, read once.
struct Look {
    locals: Vec<Local>,
    /// Each repository here, by origin hash, with its clones oldest first.
    clones: BTreeMap<String, Vec<String>>,
}

impl Orchestrators<'_> {
    fn look(&self) -> Result<Look, String> {
        let base = self.root.join("orchestrator");
        let real = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.to_path_buf())
            .join("orchestrator");
        let rows = self.store.projects().map_err(|err| err.to_string())?;
        let locals: Vec<Local> = rows
            .into_iter()
            .filter_map(|row| {
                let folder = PathBuf::from(&row.root_path);
                let rest = folder
                    .strip_prefix(&base)
                    .or_else(|_| folder.strip_prefix(&real))
                    .ok()?;
                let parts: Vec<&str> = rest
                    .components()
                    .filter_map(|part| part.as_os_str().to_str())
                    .collect();
                // As `home::orchestrator_of` reads it: `<name>` or `<profile>/<name>`.
                let fits = matches!(parts.len(), 1 | 2) && parts.iter().all(|one| plain_id(one));
                fits.then_some(Local {
                    id: row.id,
                    name: row.name,
                    folder: folder.clone(),
                    path: parts.join("/"),
                })
            })
            .collect();
        let mut clones: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for project in self
            .store
            .synced_projects()
            .map_err(|err| err.to_string())?
        {
            let orchestrator = locals.iter().any(|one| one.id == project.id);
            if let Some(hash) = project.origin_hash.filter(|hash| !hash.is_empty()) {
                if !orchestrator {
                    clones.entry(hash).or_default().push(project.id);
                }
            }
        }
        Ok(Look { locals, clones })
    }

    fn settings(&self, local: &Local) -> Result<Value, String> {
        self.store
            .orchestrator_settings(&local.folder)
            .map_err(|err| err.to_string())
    }

    /// Writes the settings that changed, and only those.
    fn keep(&self, local: &Local, was: &Value, now: &Value) -> Result<(), String> {
        for name in [SYNC_ID, LINKED, ELSEWHERE, LINKS_ON, LINKS_OFF] {
            if was[name] != now[name] {
                self.store
                    .set_orchestrator_setting(&local.folder, name, now[name].clone())
                    .map_err(|err| err.to_string())?;
            }
        }
        Ok(())
    }

    fn sync_id(&self, local: &Local, settings: &Value) -> String {
        settings[SYNC_ID]
            .as_str()
            .filter(|id| plain_id(id))
            .map(str::to_owned)
            .unwrap_or_else(|| minted(&local.path))
    }

    /// The settings as they stand after the person's own changes are read:
    /// a link made here is on, an unlink here is a tombstone, a repository
    /// that left this machine waits as elsewhere.
    fn settled(
        &self,
        local: &Local,
        seen: &Look,
    ) -> Result<(Value, BTreeSet<String>, BTreeSet<String>), String> {
        let was = self.settings(local)?;
        let mut now = was.clone();
        now[SYNC_ID] = json!(self.sync_id(local, &was));
        let mut linked = linked_of(&was);
        let mut elsewhere = strings(&was, ELSEWHERE);
        // Cloned here since: the wait is over.
        for hash in elsewhere.clone() {
            if let Some(ids) = seen.clones.get(&hash) {
                if !ids.iter().any(|id| linked.contains(id)) {
                    linked.push(ids[0].clone());
                }
                elsewhere.remove(&hash);
            }
        }
        let mut on: BTreeSet<String> = seen
            .clones
            .iter()
            .filter(|(_, ids)| ids.iter().any(|id| linked.contains(id)))
            .map(|(hash, _)| hash.clone())
            .collect();
        let mut off = strings(&was, LINKS_OFF);
        for hash in strings(&was, LINKS_ON).difference(&on) {
            match seen.clones.contains_key(hash) {
                // Unlinked here by the person.
                true => {
                    off.insert(hash.clone());
                }
                // Its clone left this machine; the link did not.
                false => {
                    elsewhere.insert(hash.clone());
                }
            }
        }
        on.extend(elsewhere.iter().cloned());
        off.retain(|hash| !on.contains(hash));
        now[LINKED] = json!(linked);
        now[ELSEWHERE] = json!(elsewhere);
        now[LINKS_ON] = json!(on);
        now[LINKS_OFF] = json!(off);
        self.keep(local, &was, &now)?;
        Ok((now, on, off))
    }

    /// A link or an unlink that arrived, applied to every clone here.
    fn link(&self, local: &Local, seen: &Look, hash: &str, on: bool) -> Result<(), String> {
        let was = self.settings(local)?;
        let mut now = was.clone();
        let mut linked = linked_of(&was);
        let mut elsewhere = strings(&was, ELSEWHERE);
        let mut links_on = strings(&was, LINKS_ON);
        let mut links_off = strings(&was, LINKS_OFF);
        let clones = seen.clones.get(hash);
        if on {
            match clones {
                Some(ids) if !ids.iter().any(|id| linked.contains(id)) => {
                    linked.push(ids[0].clone())
                }
                Some(_) => {}
                None => {
                    elsewhere.insert(hash.to_owned());
                }
            }
            links_on.insert(hash.to_owned());
            links_off.remove(hash);
        } else {
            if let Some(ids) = clones {
                linked.retain(|id| !ids.contains(id));
            }
            elsewhere.remove(hash);
            links_on.remove(hash);
            links_off.insert(hash.to_owned());
        }
        now[LINKED] = json!(linked);
        now[ELSEWHERE] = json!(elsewhere);
        now[LINKS_ON] = json!(links_on);
        now[LINKS_OFF] = json!(links_off);
        self.keep(local, &was, &now)
    }
}

impl Namespace for Orchestrators<'_> {
    fn name(&self) -> &'static str {
        "orchestrators"
    }

    fn here(&self) -> Result<Here, String> {
        let seen = self.look()?;
        let mut here = Here::new();
        for local in &seen.locals {
            // Read before anything is written: a change made offline is
            // placed by when the person made it, not by this round.
            let at = self
                .store
                .orchestrator_settings_at(&local.folder)
                .map_err(|err| err.to_string())?
                .and_then(|seconds| u64::try_from(seconds).ok())
                .map(|seconds| seconds * 1000);
            let (settings, on, off) = self.settled(local, &seen)?;
            let mut fields = BTreeMap::new();
            if fits(NAME, Some(&local.name)) {
                fields.insert(NAME.to_owned(), Seen::of(Some(local.name.clone()), None));
            }
            for (hash, state) in on
                .iter()
                .map(|hash| (hash, "on"))
                .chain(off.iter().map(|hash| (hash, "off")))
            {
                fields.insert(
                    format!("{LINK}{hash}"),
                    Seen::of(Some(state.to_owned()), at),
                );
            }
            here.insert(identity(&self.sync_id(local, &settings)), fields);
        }
        Ok(here)
    }

    fn write(&self, record: &str, field: &str, value: Option<&str>) -> Result<(), String> {
        if !fits(field, value) {
            return Ok(());
        }
        let seen = self.look()?;
        let mut found = None;
        for local in &seen.locals {
            if identity(&self.sync_id(local, &self.settings(local)?)) == record {
                found = Some(local);
                break;
            }
        }
        let Some(local) = found else {
            return Ok(());
        };
        let value = value.unwrap_or_default();
        match (field, hash_of(field)) {
            (NAME, _) => {
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
            (_, Some(hash)) => self.link(local, &seen, hash, value == "on"),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "orchestrators_tests.rs"]
mod tests;
