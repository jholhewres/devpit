//! `project.test_command` — the command a project's tests run with, read off
//! its own files, for the Tests recipe to start from. The person edits it.

use std::path::Path;

use devpit_rpc::RpcError;

/// The command, and the file it was read from. Both `None` when nothing says.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TestCommand {
    pub command: Option<String>,
    pub from: Option<String>,
}

/// The first test command Devflow's detection finds: a Makefile target is
/// what its people run, then the package manager's script, then the
/// language's own runner. One detector, so the recipe and a Devflow setup
/// never disagree about a project.
pub(crate) fn detected(root: &Path) -> TestCommand {
    let first = devpit_devflow::detect::detected(root, Default::default())
        .test
        .into_iter()
        .next();
    TestCommand {
        from: first.as_ref().map(|found| {
            let file = found.source.rsplit_once(':').map_or("", |(file, _)| file);
            file.to_owned()
        }),
        command: first.map(|found| found.command),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn project_test_command(project_id: String) -> Result<TestCommand, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let (_, root) = crate::projects::locate(&store, &project_id)?;
        Ok(detected(&root))
    })
    .await
}

#[cfg(test)]
#[path = "recipes_tests.rs"]
mod tests;
