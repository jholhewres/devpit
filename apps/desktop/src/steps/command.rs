//! The command step: your own command, as a lane of the board.

use devpit_core::Store;
use devpit_rpc::Step;
use devpit_steps as steps;

use super::Finished;

/// Runs a command step: the tests, the build, the deploy.
///
/// The output is streamed onto the card while the command works. A suite that
/// takes twenty minutes shows its first line at once — buffering would make a
/// working command look identical to a hung one for twenty minutes.
pub fn run(
    store: &Store,
    card_id: &str,
    step: &Step,
    run_id: &str,
    mut on_progress: impl FnMut(&str),
    on_pid: impl FnOnce(u32),
) -> Result<Finished, String> {
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

    let branch = crate::checkout::branch_of(step, &cwd);
    let context = super::context::context_of(
        &card,
        branch
            .as_deref()
            .map(|branch| super::context::Checkout { path: &cwd, branch }),
        Some(&project),
    )
    .for_a_command();

    // Before the command, never after: a step that commits moves HEAD, and a
    // revision read at the end would name the code this run produced rather
    // than the code it ran against.
    super::what_ran::recorded(
        store,
        run_id,
        &super::what_ran::gathered(
            &manifest.command,
            &cwd,
            &context,
            crate::checkout::needs_worktree(step.kind, &step.config),
        ),
    );

    let mut output = String::new();
    let ended = steps::run(
        &manifest.command,
        &cwd,
        &context,
        manifest.timeout_seconds.map(std::time::Duration::from_secs),
        on_pid,
        |said| {
            let line = if said.cut {
                format!("{} … (line cut)", said.text)
            } else {
                said.text.clone()
            };
            on_progress(&line);
            output.push_str(&line);
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

    if ended.output_cut {
        output.push_str("\n— the rest was dropped: this run said more than devpit keeps");
    }

    Ok(Finished {
        ok: ended.exit_code == Some(0),
        output,
        cost_usd: 0.0,
        duration_ms: ended.duration_ms,
        exit_code: ended.exit_code,
    })
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
