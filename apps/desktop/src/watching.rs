//! What each terminal is costing, in memory and in CPU.
//!
//! Two things make this worth having over reading `top`: it is grouped by the
//! pane you are looking at rather than by pid, and it counts the **whole tree**
//! under each one. An agent's cost is mostly its children — the language
//! server it started, the MCP servers it spawned — and a number that leaves
//! them out is a number saying an agent is free.
//!
//! Memory is proportional. `devpit_pty::usage` says why at length; the short
//! version is that adding RSS across a tree reports memory that does not
//! exist, by 44% on the machine this was measured on.
//!
//! CPU is a rate, so it needs two samples. The previous one is kept here
//! because it is the only state in the question: everything else is read fresh
//! from the kernel each time.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use devpit_rpc::{PaneCost, RpcError, Usage};

/// What each process had used when it was last asked, and when that was.
///
/// A rate from one sample is not a rate. The first answer for a pane therefore
/// reports no CPU rather than reporting its whole life divided by a second,
/// which would show a long-running agent at several hundred percent.
fn before() -> &'static Mutex<HashMap<u32, (u64, Instant)>> {
    static SEEN: OnceLock<Mutex<HashMap<u32, (u64, Instant)>>> = OnceLock::new();
    SEEN.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The share of a core a pane used, in tenths of a percent.
///
/// Its own function so a test can hold it: everything else here is the kernel
/// answering, and the rate is the only arithmetic in the file.
///
/// Zero in three cases, and each is a real one. No earlier sample, because a
/// rate from one sample is not a rate — dividing a long-running agent's whole
/// life by a second shows it at several hundred percent. Two samples too close
/// together, because one tick of rounding is then a whole core. And a counter
/// that went backwards, which is a pid reused between samples: an unsigned
/// subtraction there would wrap to something enormous.
fn tenths(was: u64, now_ticks: u64, elapsed: f64) -> u32 {
    if elapsed <= 0.05 || now_ticks < was {
        return 0;
    }
    let cores = (now_ticks - was) as f64 / devpit_pty::usage::ticks_per_second() as f64 / elapsed;
    (cores * 1000.0).round().max(0.0) as u32
}

/// `session.usage` — what each of a project's panes is costing right now.
///
/// Off the main thread: it reads a file per process, and a busy machine with
/// several agents open is a few hundred of them.
#[tauri::command]
#[specta::specta]
pub async fn session_usage(project_id: String) -> Result<Usage, RpcError> {
    tauri::async_runtime::spawn_blocking(move || usage_in(&project_id))
        .await
        .map_err(|err| RpcError::internal(err.to_string()))?
}

fn usage_in(project_id: &str) -> Result<Usage, RpcError> {
    let Ok(server) = crate::sessions::tmux_server() else {
        return Ok(Usage::default());
    };
    let session = devpit_tmux::Server::session_name(project_id);
    let Ok(panes) = server.running(&session) else {
        return Ok(Usage::default());
    };

    let ttys: Vec<String> = panes.iter().map(|one| one.tty.clone()).collect();
    let fronts = devpit_pty::looking(&ttys);
    /* Everything on the terminal, not only what is in front of it. A job
    backgrounded with `&` is in another process group — invisible to the
    foreground reader, and often the expensive half. Measured on a real
    pane: `sleep 600 & sleep 700` reported one process to `looking` and two
    to this. */
    let all = devpit_pty::usage::on_ttys(&ttys);
    let now = Instant::now();
    let mut seen = before().lock().map_err(|_| RpcError::internal("usage"))?;
    let mut fresh: HashMap<u32, (u64, Instant)> = HashMap::new();

    let mut costs = Vec::new();
    let mut proportional = true;
    for pane in &panes {
        // Every process under what is in front of this terminal.
        let roots = devpit_pty::usage::on_tty(&all, &pane.tty);
        // And their descendants, for anything that left the terminal behind.
        let each = devpit_pty::usage::costs(&devpit_pty::usage::tree(&roots));

        let mut memory: u64 = 0;
        let mut ticks: u64 = 0;
        for one in &each {
            memory += one.memory;
            ticks += one.ticks;
            proportional &= one.shared;
            fresh.insert(one.pid, (one.ticks, now));
        }

        // The rate, from the processes we have asked about before. A pane whose
        // processes are all new reports nothing rather than a guess.
        let (was, when) = each
            .iter()
            .filter_map(|one| seen.get(&one.pid).map(|(ticks, at)| (*ticks, *at)))
            .fold((0, None::<Instant>), |(sum, earliest), (ticks, at)| {
                (sum + ticks, Some(earliest.map_or(at, |old| old.min(at))))
            });
        let elapsed = when.map_or(0.0, |at| now.duration_since(at).as_secs_f64());

        let agent = devpit_pty::front_on(&fronts, &pane.tty)
            .and_then(|front| devpit_pty::agents::recognise(&front.argv));
        costs.push(PaneCost {
            pane_id: pane.leaf_id.clone(),
            label: agent
                .map(|one| one.label.to_owned())
                .unwrap_or_else(|| pane.command.clone()),
            agent: agent.map(|one| one.id.to_owned()),
            // Saturating rather than wrapping: four tebibytes is past
            // anything real, and a wrap would report a huge tree as tiny.
            memory_kb: memory.try_into().unwrap_or(u32::MAX),
            cpu_tenths: tenths(was, ticks, elapsed),
            processes: each.len() as u32,
        });
    }

    *seen = fresh;
    costs.sort_by(|one, two| two.memory_kb.cmp(&one.memory_kb));
    Ok(Usage {
        memory_kb: costs
            .iter()
            .map(|one| u64::from(one.memory_kb))
            .sum::<u64>()
            .try_into()
            .unwrap_or(u32::MAX),
        cpu_tenths: costs.iter().map(|one| one.cpu_tenths).sum(),
        // False anywhere is false everywhere: a total that mixes proportional
        // and resident is a total that is neither.
        proportional,
        panes: costs,
    })
}

#[cfg(test)]
#[path = "watching_tests.rs"]
mod tests;
