//! The command step: your own command, as a lane of the board.

use std::path::Path;

use devpit_core::Store;
use devpit_rpc::Step;
use devpit_steps as steps;

use super::Finished;

/// Runs a command step: the tests, the build, the deploy.
///
/// The output is streamed onto the card while the command works. A suite that
/// takes twenty minutes shows its first line at once — buffering would make a
/// working command look identical to a hung one for twenty minutes.
pub fn run(store: &Store, card_id: &str, step: &Step) -> Result<Finished, String> {
    let manifest = steps::validate(&step.config).map_err(|err| err.to_string())?;

    let card = store
        .card(card_id)
        .map_err(|err| err.to_string())?
        .ok_or("no such card")?;
    let project = store
        .project_of_card(card_id)
        .map_err(|err| err.to_string())?
        .ok_or("this card has no project on disk")?;

    // The card's own checkout, created on the first step that needs one.
    let cwd = crate::checkout::cwd_for(store, card_id, step, |_| {})?;

    let context = steps::Context {
        project: name_of(&project),
        project_path: project,
        worktree_path: cwd.display().to_string(),
        branch: card.base_ref.clone().unwrap_or_default(),
        card: card.id.clone(),
        card_title: card.title.clone(),
    };

    let mut output = String::new();
    let ended = steps::run(
        &manifest.command,
        &cwd,
        &context,
        manifest.timeout_seconds.map(std::time::Duration::from_secs),
        |line| {
            output.push_str(line);
            output.push('\n');
        },
    )
    .map_err(|err| err.to_string())?;

    if ended.timed_out {
        return Ok(Finished {
            ok: false,
            output: format!(
                "{output}\n— killed after {}s, the timeout this step declares",
                manifest.timeout_seconds.unwrap_or_default()
            ),
            cost_usd: 0.0,
            duration_ms: ended.duration_ms,
            exit_code: None,
        });
    }

    Ok(Finished {
        ok: ended.exit_code == Some(0),
        output,
        cost_usd: 0.0,
        duration_ms: ended.duration_ms,
        exit_code: ended.exit_code,
    })
}

/// The last segment of a path, which is what a project is called.
fn name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned())
}
