//! What a project is called and how it looks in the rail: its name, its
//! group, its colour. The app's own labels — nothing here touches the folder.

use devpit_rpc::{ErrorCode, ProjectList, RpcError};

use crate::projects::{project_list_now, store};

/// `project.rename` — what this project is called in devpit.
///
/// The name is the app's, not git's: the folder on disk keeps whatever it was
/// called, because renaming somebody's checkout is not a thing a list should
/// do to make its own rows read better.
#[tauri::command]
#[specta::specta]
pub async fn project_rename(project_id: String, name: String) -> Result<ProjectList, RpcError> {
    crate::off_main::blocking(move || project_rename_now(project_id, name)).await
}

/// [`project_rename`], on the calling thread.
pub(crate) fn project_rename_now(
    project_id: String,
    name: String,
) -> Result<ProjectList, RpcError> {
    let wanted = name.trim();
    if wanted.is_empty() {
        return Err(RpcError::new(ErrorCode::Invalid, "a project needs a name"));
    }
    if wanted.chars().count() > 120 {
        return Err(RpcError::new(ErrorCode::Invalid, "that name is too long"));
    }

    let store = store()?;
    if !store.rename_project(&project_id, wanted)? {
        return Err(RpcError::new(ErrorCode::NotFound, "no such project"));
    }
    project_list_now()
}

/// `project.edit` — a project's name, group and mark, from the dialog that
/// sets them together.
///
/// An empty group, icon or colour clears it. The colour is `#rrggbb` and
/// nothing else, because it is written into a style; the icon is short, because
/// it is either one of the app's own names or a single emoji.
#[tauri::command]
#[specta::specta]
pub async fn project_edit(
    project_id: String,
    name: String,
    group: Option<String>,
    icon: Option<String>,
    color: Option<String>,
) -> Result<ProjectList, RpcError> {
    crate::off_main::blocking(move || project_edit_now(project_id, name, group, icon, color)).await
}

/// [`project_edit`], on the calling thread.
pub(crate) fn project_edit_now(
    project_id: String,
    name: String,
    group: Option<String>,
    icon: Option<String>,
    color: Option<String>,
) -> Result<ProjectList, RpcError> {
    let wanted = name.trim();
    if wanted.is_empty() {
        return Err(RpcError::new(ErrorCode::Invalid, "a project needs a name"));
    }
    if wanted.chars().count() > 120 {
        return Err(RpcError::new(ErrorCode::Invalid, "that name is too long"));
    }
    let given = |value: &Option<String>| {
        value
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    };
    let group = given(&group);
    let icon = given(&icon);
    let color = given(&color);
    if group
        .as_deref()
        .is_some_and(|group| group.chars().count() > 60)
    {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "that group name is too long",
        ));
    }
    if icon
        .as_deref()
        .is_some_and(|icon| icon.chars().count() > 32)
    {
        return Err(RpcError::new(ErrorCode::Invalid, "that is not an icon"));
    }
    if color.as_deref().is_some_and(|color| !is_hex_colour(color)) {
        return Err(RpcError::new(ErrorCode::Invalid, "a colour is #rrggbb"));
    }

    let store = store()?;
    if !store.edit_project(
        &project_id,
        wanted,
        group.as_deref(),
        icon.as_deref(),
        color.as_deref(),
    )? {
        return Err(RpcError::new(ErrorCode::NotFound, "no such project"));
    }
    project_list_now()
}

/// `project.group_rename` — a group's name, changed on every project in it.
/// An empty name takes them out of the group.
#[tauri::command]
#[specta::specta]
pub async fn project_group_rename(from: String, to: String) -> Result<ProjectList, RpcError> {
    crate::off_main::blocking(move || project_group_rename_now(from, to)).await
}

/// [`project_group_rename`], on the calling thread.
pub(crate) fn project_group_rename_now(from: String, to: String) -> Result<ProjectList, RpcError> {
    let to = to.trim();
    if to.chars().count() > 60 {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "that group name is too long",
        ));
    }
    store()?.rename_group(&from, (!to.is_empty()).then_some(to))?;
    project_list_now()
}

fn is_hex_colour(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|letter| letter.is_ascii_hexdigit())
}
