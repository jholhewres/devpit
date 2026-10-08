//! A project's secrets, put where its sessions read them without passing
//! through a chat.
//!
//! Keys pasted into the orchestrator's chat ended up in transcripts and had to
//! be rotated. Here the value goes from a password field straight into the
//! project's `.env`, readable by the person alone (0600) and kept out of git
//! in this clone (`info/exclude`, never the versioned `.gitignore`). The
//! agent sees only the name; devpit keeps the name and the date, never the
//! value, and never says it in an error.

use std::path::Path;

use devpit_rpc::{ErrorCode, RpcError, Secret, SecretList};

/// The file the values go in, at the project's root.
const FILE: &str = ".env";
/// The most of an existing `.env` read: a bigger one is not a `.env`.
const CEILING: u64 = 1024 * 1024;
const LONGEST_VALUE: usize = 8 * 1024;

/// A variable name a shell and every `.env` reader take as it is.
pub(crate) fn plain_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_uppercase() || first == '_')
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && name.len() <= 64
}

/// `env` with `name` set to `value`: its own line replaced where it was, or
/// added at the end. Every other line is kept as it was.
pub(crate) fn with_secret(env: &str, name: &str, value: &str) -> String {
    let line = format!("{name}={}", quoted(value));
    let mut found = false;
    let mut out: Vec<String> = env
        .lines()
        .map(|was| {
            let bare = was.trim_start().trim_start_matches("export ").trim_start();
            if !found
                && bare
                    .strip_prefix(name)
                    .is_some_and(|rest| rest.starts_with('='))
            {
                found = true;
                line.clone()
            } else {
                was.to_owned()
            }
        })
        .collect();
    if !found {
        out.push(line);
    }
    format!("{}\n", out.join("\n"))
}

/// A value as `.env` readers take it back: bare when it can be, else quoted.
fn quoted(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./:@+=,".contains(c))
    {
        return value.to_owned();
    }
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn refused(why: &str) -> RpcError {
    RpcError::new(ErrorCode::Invalid, why.to_owned())
}

/// Puts `value` in the project's `.env` as `name`.
pub(crate) fn keep(
    root: &Path,
    kept: &Path,
    name: &str,
    value: &str,
) -> Result<SecretList, RpcError> {
    if !plain_name(name) {
        return Err(refused(
            "a name is capitals, digits and _, like JEV_API_KEY",
        ));
    }
    if value.is_empty() || value.len() > LONGEST_VALUE || value.contains(['\n', '\r', '\0']) {
        return Err(refused("a secret is one line, at most 8 KB"));
    }
    let root = root
        .canonicalize()
        .map_err(|err| RpcError::internal(err.to_string()))?;
    let file = root.join(FILE);
    // Through a symlink only to somewhere inside the project.
    if let Ok(real) = file.canonicalize() {
        if !real.starts_with(&root) {
            return Err(refused("the project's .env points outside the project"));
        }
    }
    let mut was = String::new();
    if let Ok(opened) = std::fs::File::open(&file) {
        std::io::Read::read_to_string(&mut std::io::Read::take(opened, CEILING), &mut was)
            .map_err(|_| refused("the project's .env could not be read"))?;
    }
    if !devpit_git::ignored(&root, FILE).map_err(|err| RpcError::internal(err.to_string()))? {
        devpit_git::exclude(&root, FILE).map_err(|err| RpcError::internal(err.to_string()))?;
    }
    let temporary = root.join(".env.devpit-writing");
    write_private(&temporary, &with_secret(&was, name, value))?;
    std::fs::rename(&temporary, &file).map_err(|err| {
        let _ = std::fs::remove_file(&temporary);
        RpcError::internal(err.to_string())
    })?;
    let mut list = listed(kept);
    list.retain(|one| one.name != name);
    list.push(Secret {
        name: name.to_owned(),
        set_at: devpit_core::reports::now() as f64,
    });
    list.sort_by(|a, b| a.name.cmp(&b.name));
    let text =
        serde_json::to_string_pretty(&list).map_err(|err| RpcError::internal(err.to_string()))?;
    if let Some(parent) = kept.parent() {
        std::fs::create_dir_all(parent).map_err(|err| RpcError::internal(err.to_string()))?;
    }
    std::fs::write(kept, text).map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(SecretList {
        secrets: list,
        file: file.display().to_string(),
    })
}

/// The names kept, as `secrets.json` lists them.
pub(crate) fn listed(kept: &Path) -> Vec<Secret> {
    std::fs::read_to_string(kept)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Written readable by the person alone, from its first byte.
fn write_private(path: &Path, text: &str) -> Result<(), RpcError> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options
        .open(path)
        .map_err(|err| RpcError::internal(err.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = file.set_permissions(std::fs::Permissions::from_mode(0o600));
    }
    std::io::Write::write_all(&mut file, text.as_bytes())
        .map_err(|err| RpcError::internal(err.to_string()))
}

/// `project.secrets` — the names kept for a project, and where.
#[tauri::command]
#[specta::specta]
pub async fn project_secrets(project_id: String) -> Result<SecretList, RpcError> {
    crate::off_main::blocking(move || {
        let root = crate::roots::root_of(&project_id, None)?;
        Ok(SecretList {
            secrets: listed(&crate::projects::project_home(&project_id)?.secrets()),
            file: root.join(FILE).display().to_string(),
        })
    })
    .await
}

/// `project.secret_set` — puts one in the project's `.env`.
#[tauri::command]
#[specta::specta]
pub async fn project_secret_set(
    project_id: String,
    name: String,
    value: String,
) -> Result<SecretList, RpcError> {
    crate::off_main::blocking(move || {
        let root = crate::roots::root_of(&project_id, None)?;
        keep(
            &root,
            &crate::projects::project_home(&project_id)?.secrets(),
            name.trim(),
            &value,
        )
    })
    .await
}

#[cfg(test)]
#[path = "vault_tests.rs"]
mod tests;
