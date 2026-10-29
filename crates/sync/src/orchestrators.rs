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
/// The repository each linked project is a clone of, so a link outlives the
/// clone being forgotten.
const LINK_ORIGINS: &str = "link_origins";

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
    /// What its id is minted from: its folder's name, or its path under
    /// `orchestrator/` when two here share a name.
    path: String,
    /// The id it keeps, or the one it is minted now.
    sync_id: String,
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

fn last(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_owned()
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

/// A list of strings in the settings: links as a `Vec`, in the order the
/// person made them; the rest as sets.
fn strings<C: FromIterator<String> + Default>(settings: &Value, name: &str) -> C {
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
        let mut locals: Vec<Local> = rows
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
                let shaped = matches!(parts.len(), 1 | 2) && parts.iter().all(|one| plain_id(one));
                let path = parts.join("/");
                shaped.then_some(Local {
                    id: row.id,
                    name: row.name,
                    folder,
                    path,
                    sync_id: String::new(),
                })
            })
            .collect();
        // Minted from the folder's name, which an account's folder above it
        // does not change; from the whole path only where two share a name.
        let names: Vec<String> = locals.iter().map(|one| last(&one.path)).collect();
        for (at, local) in locals.iter_mut().enumerate() {
            let name = &names[at];
            if names.iter().filter(|one| *one == name).count() == 1 {
                local.path = name.clone();
            }
        }
        // An id kept is kept; one minted now never takes an id another
        // orchestrator here already holds.
        let kept: Vec<Option<String>> = locals
            .iter()
            .map(|one| {
                let settings = self.store.orchestrator_settings(&one.folder).ok()?;
                settings[SYNC_ID]
                    .as_str()
                    .filter(|id| plain_id(id))
                    .map(str::to_owned)
            })
            .collect();
        let mut taken: BTreeSet<String> = kept.iter().flatten().cloned().collect();
        for (local, kept) in locals.iter_mut().zip(kept) {
            local.sync_id = kept.unwrap_or_else(|| {
                let mut id = minted(&local.path);
                if taken.contains(&id) {
                    id = minted(&format!("/{}", local.folder.display()));
                }
                taken.insert(id.clone());
                id
            });
        }
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
        for name in [
            SYNC_ID,
            LINKED,
            ELSEWHERE,
            LINKS_ON,
            LINKS_OFF,
            LINK_ORIGINS,
        ] {
            if was[name] != now[name] {
                self.store
                    .set_orchestrator_setting(&local.folder, name, now[name].clone())
                    .map_err(|err| err.to_string())?;
            }
        }
        Ok(())
    }

    /// The settings as they stand after the person's own changes are read:
    /// a link made here is on, an unlink here is a tombstone, a repository
    /// that left this machine waits as elsewhere.
    fn settled(
        &self,
        local: &Local,
        look: &Look,
    ) -> Result<(BTreeSet<String>, BTreeSet<String>), String> {
        let was = self.settings(local)?;
        let mut now = was.clone();
        now[SYNC_ID] = json!(local.sync_id);
        let mut linked = strings::<Vec<String>>(&was, LINKED);
        let mut elsewhere: BTreeSet<String> = strings(&was, ELSEWHERE);
        // A linked clone forgotten here: its link moves to another clone, or
        // waits as elsewhere. The person unlinked nothing.
        let live: BTreeMap<&String, &String> = look
            .clones
            .iter()
            .flat_map(|(hash, ids)| ids.iter().map(move |id| (id, hash)))
            .collect();
        let mut origins = was[LINK_ORIGINS].as_object().cloned().unwrap_or_default();
        for id in linked.clone() {
            if let Some(hash) = live.get(&id) {
                origins.insert(id, json!(hash));
                continue;
            }
            let Some(hash) = origins
                .remove(&id)
                .and_then(|hash| hash.as_str().map(str::to_owned))
            else {
                continue;
            };
            linked.retain(|one| one != &id);
            match look.clones.get(&hash) {
                Some(ids) if !ids.iter().any(|one| linked.contains(one)) => {
                    linked.push(ids[0].clone());
                    origins.insert(ids[0].clone(), json!(hash));
                }
                Some(_) => {}
                None => {
                    elsewhere.insert(hash);
                }
            }
        }
        origins.retain(|id, _| linked.contains(id));
        now[LINK_ORIGINS] = Value::Object(origins);
        // Cloned here since: the wait is over.
        for hash in elsewhere.clone() {
            if let Some(ids) = look.clones.get(&hash) {
                if !ids.iter().any(|id| linked.contains(id)) {
                    linked.push(ids[0].clone());
                }
                elsewhere.remove(&hash);
            }
        }
        let mut on: BTreeSet<String> = look
            .clones
            .iter()
            .filter(|(_, ids)| ids.iter().any(|id| linked.contains(id)))
            .map(|(hash, _)| hash.clone())
            .collect();
        let mut off: BTreeSet<String> = strings(&was, LINKS_OFF);
        for hash in strings::<BTreeSet<String>>(&was, LINKS_ON).difference(&on) {
            match look.clones.contains_key(hash) {
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
        Ok((on, off))
    }

    /// A link or an unlink that arrived, applied to every clone here.
    fn link(&self, local: &Local, look: &Look, hash: &str, on: bool) -> Result<(), String> {
        let was = self.settings(local)?;
        let mut now = was.clone();
        let mut linked = strings::<Vec<String>>(&was, LINKED);
        let mut elsewhere: BTreeSet<String> = strings(&was, ELSEWHERE);
        let mut links_on: BTreeSet<String> = strings(&was, LINKS_ON);
        let mut links_off: BTreeSet<String> = strings(&was, LINKS_OFF);
        let clones = look.clones.get(hash);
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
        let look = self.look()?;
        let mut here = Here::new();
        for local in &look.locals {
            // Read before anything is written: a change made offline is
            // placed by when the person made it, not by this round.
            let at = self
                .store
                .orchestrator_settings_at(&local.folder)
                .map_err(|err| err.to_string())?
                .and_then(|seconds| u64::try_from(seconds).ok())
                .map(|seconds| seconds * 1000);
            let (on, off) = self.settled(local, &look)?;
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
            here.insert(identity(&local.sync_id), fields);
        }
        Ok(here)
    }

    fn write(&self, record: &str, field: &str, value: Option<&str>) -> Result<(), String> {
        if !fits(field, value) {
            return Ok(());
        }
        let look = self.look()?;
        let Some(local) = look
            .locals
            .iter()
            .find(|local| identity(&local.sync_id) == record)
        else {
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
            (_, Some(hash)) => self.link(local, &look, hash, value == "on"),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "orchestrators_tests.rs"]
mod tests;
