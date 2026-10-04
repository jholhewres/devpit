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

/// The project's own test entry first: a Makefile target is what its people
/// run, then the package manager's script, then cargo. A JS runner writes its
/// report where the step reads it.
pub(crate) fn detected(root: &Path) -> TestCommand {
    let read = |name: &str| std::fs::read_to_string(root.join(name)).ok();
    let found = |command: String, from: &str| TestCommand {
        command: Some(command),
        from: Some(from.to_owned()),
    };
    if read("Makefile").is_some_and(|make| make.lines().any(|line| line.starts_with("test:"))) {
        return found("make test".to_owned(), "Makefile");
    }
    if let Some(package) =
        read("package.json").and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
    {
        let has = |name: &str| {
            ["dependencies", "devDependencies"]
                .iter()
                .any(|deps| package.get(deps).and_then(|deps| deps.get(name)).is_some())
        };
        let exec = if root.join("pnpm-lock.yaml").exists() {
            "pnpm exec"
        } else if root.join("yarn.lock").exists() {
            "yarn"
        } else {
            "npx"
        };
        let report = "--outputFile=\"$DEVPIT_REPORT_DIR/tests.json\"";
        if has("vitest") {
            return found(
                format!("{exec} vitest run --reporter=default --reporter=json {report}"),
                "package.json",
            );
        }
        if has("jest") {
            return found(format!("{exec} jest --json {report}"), "package.json");
        }
        if package
            .get("scripts")
            .and_then(|scripts| scripts.get("test"))
            .is_some()
        {
            let manager = exec
                .split(' ')
                .next()
                .filter(|one| *one != "npx")
                .unwrap_or("npm");
            return found(format!("{manager} test"), "package.json");
        }
    }
    if root.join("Cargo.toml").exists() {
        return found("cargo test".to_owned(), "Cargo.toml");
    }
    TestCommand {
        command: None,
        from: None,
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
