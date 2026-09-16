//! The guards, as a command.
//!
//! Each is written so that it fails when removed — a guard only ever seen
//! passing proves nothing. One guard per module, with the test that catches
//! the thing it exists to catch next to it.
//!
//! Runs as `cargo xtask check`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod agent_boundary;
mod csp;
mod dead_controls;
mod home_paths;
mod naming;
mod platform_window;
mod ratchet;
mod reseed;
mod shell_boundary;

fn main() -> ExitCode {
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "check".to_owned());

    match command.as_str() {
        "check" => check(),
        "ceilings" => ceilings(),
        "controls" => controls(),
        other => {
            eprintln!("unknown command: {other}\n\nusage: cargo xtask check | ceilings | controls");
            ExitCode::FAILURE
        }
    }
}

/// Rewrites the size ceilings from what the tree measures now.
fn ceilings() -> ExitCode {
    match reseed::reseed(&workspace_root()) {
        Ok(count) => {
            println!("ceilings: {count} files");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("could not write ceilings: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Rewrites the dead-control budgets from what the tree has now.
///
/// Tightening only, for the same reason as the ceilings: a command that can
/// raise a budget is the way around the guard it serves.
fn controls() -> ExitCode {
    match dead_controls::reseed(&workspace_root()) {
        Ok(count) => {
            println!("dead controls: {count} left");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("could not write the budgets: {error}");
            ExitCode::FAILURE
        }
    }
}

fn check() -> ExitCode {
    let root = workspace_root();
    let mut findings = Vec::new();

    findings.extend(shell_boundary::core_does_not_know_the_shell(&root));
    findings.extend(platform_window::platform_window_matches_the_base(&root));
    findings.extend(naming::nothing_is_named_after_nothing(&root));
    findings.extend(ratchet::files_only_get_shorter(&root));
    findings.extend(agent_boundary::only_one_crate_drives_the_agent(&root));
    findings.extend(dead_controls::a_control_either_works_or_goes(&root));
    findings.extend(home_paths::paths_come_from_home(&root));
    findings.extend(csp::the_csp_forbids_what_the_app_never_needs(&root));

    if findings.is_empty() {
        println!("guards: ok");
        return ExitCode::SUCCESS;
    }

    eprintln!("\n{} violation(s):\n", findings.len());
    for finding in &findings {
        eprintln!("  {finding}");
    }
    eprintln!();
    ExitCode::FAILURE
}

/// A violation says where, not just that there was one.
///
/// "The guard failed" sends someone looking; file and line send someone
/// fixing. The difference shows when the reader is not the author.
pub struct Finding {
    pub file: PathBuf,
    pub line: usize,
    pub what: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{} — {}", self.file.display(), self.line, self.what)
    }
}

/// The repository root, which every guard walks from.
///
/// `CARGO_MANIFEST_DIR` points at `xtask/`; the root is its parent. Deriving
/// it from the current directory would break when the command is run from
/// inside a crate.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}
