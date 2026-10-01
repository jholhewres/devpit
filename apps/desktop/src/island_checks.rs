//! Whether a session's branch has a pull request and how its checks are
//! doing, asked of `gh` — only for the session open on the island, and not
//! more than once a minute.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use devpit_rpc::{ErrorCode, IslandChecks, IslandPull, RpcError};
use serde::Deserialize;

/// How long an answer is kept before `gh` is asked again.
const FRESH: Duration = Duration::from_secs(60);
/// How long `gh` may take before the island stops waiting for it.
const GIVE_UP: Duration = Duration::from_secs(15);

/// What a list of check results adds up to: one failing is failing, one
/// still going is running, and only all done and fine is passing.
pub(crate) fn rollup<'a>(
    results: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Option<&'static str> {
    let mut any = false;
    let mut running = false;
    for (status, conclusion) in results {
        any = true;
        let word = if conclusion.is_empty() {
            status
        } else {
            conclusion
        };
        match word.to_ascii_uppercase().as_str() {
            "FAILURE" | "ERROR" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED"
            | "STARTUP_FAILURE" => return Some("failing"),
            "SUCCESS" | "NEUTRAL" | "SKIPPED" | "COMPLETED" => {}
            _ => running = true,
        }
    }
    any.then_some(if running { "running" } else { "passing" })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pull {
    number: u32,
    state: String,
    title: String,
    url: String,
    #[serde(default)]
    status_check_rollup: Vec<Check>,
}

#[derive(Deserialize)]
struct Check {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    conclusion: Option<String>,
}

impl Check {
    fn said(&self) -> (&str, &str) {
        let status = self
            .status
            .as_deref()
            .or(self.state.as_deref())
            .unwrap_or("");
        (status, self.conclusion.as_deref().unwrap_or(""))
    }
}

/// `gh` in a folder, given up on when it takes too long.
fn gh(root: &Path, args: &[&str]) -> Option<String> {
    let root = root.to_path_buf();
    let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    let (tell, hear) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let ran = devpit_pty::host_env::command("gh")
            .args(&args)
            .current_dir(&root)
            .stdin(std::process::Stdio::null())
            .output();
        let _ = tell.send(ran);
    });
    let ran = hear.recv_timeout(GIVE_UP).ok()?.ok()?;
    ran.status
        .success()
        .then(|| String::from_utf8_lossy(&ran.stdout).into_owned())
}

fn asked(root: &Path, branch: &str) -> IslandChecks {
    let pull = gh(
        root,
        &[
            "pr",
            "view",
            branch,
            "--json",
            "number,state,title,url,statusCheckRollup",
        ],
    )
    .and_then(|said| serde_json::from_str::<Pull>(&said).ok());
    let checks = match &pull {
        Some(pull) => rollup(pull.status_check_rollup.iter().map(Check::said)),
        None => gh(
            root,
            &[
                "run",
                "list",
                "--branch",
                branch,
                "--limit",
                "1",
                "--json",
                "status,conclusion",
            ],
        )
        .and_then(|said| serde_json::from_str::<Vec<Check>>(&said).ok())
        .and_then(|runs| rollup(runs.iter().map(Check::said))),
    };
    IslandChecks {
        branch: branch.to_owned(),
        checks: checks.map(str::to_owned),
        pull: pull.map(|pull| IslandPull {
            number: pull.number,
            state: pull.state,
            title: pull.title,
            url: pull.url,
        }),
    }
}

type Kept = HashMap<(PathBuf, String), (Instant, IslandChecks)>;
static KEPT: Mutex<Option<Kept>> = Mutex::new(None);

/// `island.checks` — the pull request and checks of a session's branch.
#[tauri::command]
#[specta::specta]
pub async fn island_checks(session_id: String) -> Result<IslandChecks, RpcError> {
    crate::off_main::blocking(move || {
        let session = crate::island_feed::now()
            .into_iter()
            .find(|one| one.session_id == session_id)
            .ok_or_else(|| {
                RpcError::new(ErrorCode::NotFound, "that session has left the island")
            })?;
        let root =
            PathBuf::from(session.root.ok_or_else(|| {
                RpcError::new(ErrorCode::NotFound, "that session is in no project")
            })?);
        let branch = devpit_git::repo_state(&root, &[], false)
            .map_err(|err| RpcError::internal(err.to_string()))?
            .branch;
        if !devpit_git::plain_branch(&branch) || branch == "HEAD" {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "that checkout is on no branch",
            ));
        }
        let key = (root.clone(), branch.clone());
        if let Some((at, kept)) = KEPT
            .lock()
            .ok()
            .and_then(|held| held.as_ref()?.get(&key).cloned())
        {
            if at.elapsed() < FRESH {
                return Ok(kept);
            }
        }
        let fresh = asked(&root, &branch);
        if let Ok(mut held) = KEPT.lock() {
            held.get_or_insert_with(HashMap::new)
                .insert(key, (Instant::now(), fresh.clone()));
        }
        Ok(fresh)
    })
    .await
}

#[cfg(test)]
#[path = "island_checks_tests.rs"]
mod tests;
