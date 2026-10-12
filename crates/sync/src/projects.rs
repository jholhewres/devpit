//! A project's settings on every machine that has it: its name and group in
//! the rail, its mark, its plugins, and whether Decisions may read it.
//!
//! A project is the same project on two machines when it is a clone of the
//! same repository: its record is `origin:<hash of the normalised git
//! origin>`, the hash devpit already keeps. A project without an origin has
//! nothing another machine could match, and stays here; so does a second
//! clone of one repository on the same machine, whose settings are its own —
//! the oldest clone, by when it was added and then by id, carries the
//! repository's.

use std::collections::{BTreeMap, BTreeSet};

use devpit_core::{preference as key, Store, SyncedProject};

use crate::space::{Here, Namespace, Seen};

pub const NAME: &str = "name";
pub const GROUP: &str = "group";
pub const ICON: &str = "icon";
pub const COLOR: &str = "color";
/// `true` when the project's state is never sent to Decisions.
pub const DECISIONS_NEVER: &str = "decisions.never";
/// `plugin.<id>`: `on`, `off` or `removed`.
pub const PLUGIN: &str = "plugin.";

/// A project's record, by its repository.
pub fn identity(origin_hash: &str) -> String {
    format!("origin:{origin_hash}")
}

/// The projects that carry their repository's settings, by record.
pub fn carriers(projects: &[SyncedProject]) -> BTreeMap<String, &SyncedProject> {
    let mut carriers = BTreeMap::new();
    let mut by_age: Vec<&SyncedProject> = projects.iter().collect();
    by_age.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));
    for project in by_age {
        if let Some(hash) = project
            .origin_hash
            .as_deref()
            .filter(|hash| !hash.is_empty())
        {
            carriers.entry(identity(hash)).or_insert(project);
        }
    }
    carriers
}

/// Whether `value` may be written as `field`: the rules the edit dialog
/// keeps, since a value from elsewhere is checked here again.
pub fn fits(field: &str, value: Option<&str>) -> bool {
    match (field, value) {
        (NAME, Some(name)) => !name.trim().is_empty() && name.chars().count() <= 120,
        (NAME, None) => false,
        (GROUP, group) => {
            group.is_none_or(|group| !group.trim().is_empty() && group.chars().count() <= 60)
        }
        (ICON, icon) => icon.is_none_or(|icon| !icon.is_empty() && icon.chars().count() <= 32),
        (COLOR, color) => color.is_none_or(|color| {
            color.len() == 7
                && color.starts_with('#')
                && color[1..].chars().all(|c| c.is_ascii_hexdigit())
        }),
        (DECISIONS_NEVER, Some(flag)) => matches!(flag, "true" | "false"),
        (field, Some(state)) => {
            plugin_of(field).is_some() && matches!(state, "on" | "off" | "removed")
        }
        _ => false,
    }
}

/// The plugin a `plugin.<id>` field names, when the id is a plain one.
pub fn plugin_of(field: &str) -> Option<&str> {
    field.strip_prefix(PLUGIN).filter(|id| {
        (1..=64).contains(&id.len())
            && id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_'))
    })
}

/// The project settings namespace, over this machine's store.
pub struct Projects<'a> {
    pub store: &'a Store,
}

impl Projects<'_> {
    fn never(&self) -> Result<(BTreeSet<String>, Option<u64>), String> {
        let kept = self
            .store
            .preference_stamped(key::DECISIONS_OPTED_OUT)
            .map_err(|err| err.to_string())?;
        let at = kept
            .as_ref()
            .and_then(|(_, at)| u64::try_from(*at).ok())
            .map(|seconds| seconds * 1000);
        let ids = kept
            .and_then(|(list, _)| serde_json::from_str::<Vec<String>>(&list).ok())
            .unwrap_or_default();
        Ok((ids.into_iter().collect(), at))
    }

    fn carrier(&self, record: &str) -> Result<Option<SyncedProject>, String> {
        let projects = self
            .store
            .synced_projects()
            .map_err(|err| err.to_string())?;
        Ok(carriers(&projects)
            .get(record)
            .map(|project| (*project).clone()))
    }
}

impl Namespace for Projects<'_> {
    fn name(&self) -> &'static str {
        "projects"
    }

    fn here(&self) -> Result<Here, String> {
        let projects = self
            .store
            .synced_projects()
            .map_err(|err| err.to_string())?;
        let (never, never_at) = self.never()?;
        let mut here = Here::new();
        for (record, project) in carriers(&projects) {
            let mut fields = BTreeMap::new();
            let mut put = |field: &str, value: Option<String>, at: Option<u64>| {
                if fits(field, value.as_deref()) {
                    fields.insert(field.to_owned(), Seen::of(value, at));
                }
            };
            put(NAME, Some(project.name.clone()), None);
            put(GROUP, project.group.clone(), None);
            put(ICON, project.icon.clone(), None);
            put(COLOR, project.color.clone(), None);
            put(
                DECISIONS_NEVER,
                Some(never.contains(&project.id).to_string()),
                never_at,
            );
            for row in self
                .store
                .plugin_rows(&project.id)
                .map_err(|err| err.to_string())?
            {
                let state = match (row.installed, row.enabled) {
                    (false, _) => "removed",
                    (true, true) => "on",
                    (true, false) => "off",
                };
                let at = u64::try_from(row.updated_at)
                    .ok()
                    .map(|seconds| seconds * 1000);
                put(
                    &format!("{PLUGIN}{}", row.plugin_id),
                    Some(state.to_owned()),
                    at,
                );
            }
            here.insert(record, fields);
        }
        Ok(here)
    }

    fn write(&self, record: &str, field: &str, value: Option<&str>) -> Result<(), String> {
        if !fits(field, value) {
            return Ok(());
        }
        let Some(project) = self.carrier(record)? else {
            return Ok(());
        };
        let failed = |err: devpit_core::StoreError| err.to_string();
        let mut edited = project.clone();
        match field {
            NAME => edited.name = value.unwrap_or_default().trim().to_owned(),
            GROUP => edited.group = value.map(str::to_owned),
            ICON => edited.icon = value.map(str::to_owned),
            COLOR => edited.color = value.map(str::to_owned),
            DECISIONS_NEVER => {
                let (mut never, _) = self.never()?;
                never.remove(&project.id);
                if value == Some("true") {
                    never.insert(project.id.clone());
                }
                let list: Vec<&String> = never.iter().collect();
                return self
                    .store
                    .set_preference(
                        key::DECISIONS_OPTED_OUT,
                        &serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_owned()),
                    )
                    .map_err(failed);
            }
            plugin => {
                let Some(id) = plugin_of(plugin) else {
                    return Ok(());
                };
                let (installed, enabled) = match value {
                    Some("on") => (true, true),
                    Some("off") => (true, false),
                    _ => (false, false),
                };
                return self
                    .store
                    .set_plugin_row(&project.id, id, installed, enabled)
                    .map_err(failed);
            }
        }
        self.store
            .edit_project(
                &project.id,
                &edited.name,
                edited.group.as_deref(),
                edited.icon.as_deref(),
                edited.color.as_deref(),
            )
            .map(|_| ())
            .map_err(failed)
    }
}

#[cfg(test)]
#[path = "projects_tests.rs"]
mod tests;
