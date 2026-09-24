# Architecture

Where things live, and the rules the build refuses to let you break.

## Layout

```
crates/core      domain, SQLite store, event bus
crates/git       status, worktrees, clone, log
crates/pty       pty handling, ring buffer, coalescing
crates/rpc       the contract; frontend types are generated from it
crates/tmux      the sessions that outlive the window
crates/agentcli  the agent catalogue, headless turns, hooks
crates/steps     what a column runs, and the runner that runs it
apps/desktop     the shell — the only crate allowed to know it
web              React + Vite + xterm.js
xtask            the architectural guards
```

## The rules that have teeth

`cargo xtask check` runs thirteen guards, each one a rule that failed silently
before it existed. It names the file and the line. They are not style
preferences:

| | |
|---|---|
| shell boundary | nothing under `crates/` imports the shell |
| agent boundary | one crate drives the agent CLI |
| uncalled commands | a command in the contract is called from `web/src`, or listed with a reason in `xtask/uncalled-commands.txt` |
| dead controls | a button with a handler, or no button — a budget per file that only shrinks |
| size ratchet | a file past its ceiling in `xtask/ceilings.txt` is split, never given a bigger ceiling |
| naming | nothing is named after nothing |
| home paths | paths come from the state root, not from `$HOME` by hand |
| csp | the window's policy allows no remote script and no remote frame |
| versions | the version has one source |
| packaging | the bundle ships what it says it ships |
| release workflow | the release workflow keeps its promises |
| platform window | the platform window matches the base |
| tmux survives | nothing in `apps/desktop` names `kill_server` |

Two more rules are enforced by tests rather than guards: **types come from
Rust** (`web/src/gen/` is generated, and a stale contract fails `make test`),
and **every path is resolved through symlinks and checked against the project
root** — this process runs terminals and writes files, so reaching it is
reaching the machine.

## A card follows its sessions

A card knows what is working on it from links that already live elsewhere and
from what its agents say — never from a copy of their state in a table, which
would still say `working` after the app had closed.

- **Links are rows.** A card's terminal tab (`tab_card_<id>`), a run's own
  session (`run.session_id`, with the folder it ran in, `run.cwd`), a background
  session (`session_link`, with its `cwd`) and a conversation about the card
  (`card_chat`). A card's sessions are these, joined when read.
- **State is heard.** Hooks from panes, headless turns and background sessions,
  runs starting and ending, chat turns and the process table all land in one
  in-memory registry, stamped by one counter, so a late word never undoes a
  newer one. After a restart it is empty, so the app rebuilds what it can on
  its own thread: for the project you were last in, tmux and the process table
  say which card panes have an agent in front, and those read as open until
  the agent says more. Other projects are rebuilt when they are opened. What
  an agent was *doing* — working, waiting, done — waits for its next hook.
- **The window hears it on `card:happening`.** One event per change, with the
  card's sessions and what they add up to. The tile's dot and the open card's
  Sessions section read it.
- **A conversation runs where its session ran.** Its folder is fixed when it
  begins or takes a session in, and no turn makes a checkout.

## What a run writes down

A run is a row, opened when a card lands on a lane with a step and closed when
the step returns. It holds the state (`running`, `ok`, `failed`, `cancelled`,
`lost`), the output as it came, the exit code for a command, the cost and the
duration for an agent turn, the lane it came from, the folder it ran in and the
session it ran as. The card shows the last one; what a run cost is on the card,
and what your plan has left is on the Usage page, which reads the CLI's own
credentials and asks Anthropic.

A `session` step is not limited to one at a time. Each one starts a detached
tmux session for its card and writes a link to it; what is limited to one is
the target terminal, which is the session you are looking at.

## How an update goes in

The check is `tauri-plugin-updater` against a signed feed, and the bytes are
verified against the release public key before anything is written. What
happens then depends on what this build is:

- an **AppImage** is installed over itself and the window restarts. The
  terminals are tmux sessions on a server devpit never kills, so they are still
  there when it comes back — no restore step, because nothing was lost.
- a **`.deb`** is checked again, kept `0644` in a cache directory, and shown to
  you as a command to run. devpit never runs it: installing a system package
  means root, and devpit does not ask for root on anybody's behalf.

From the moment an update is ready, nothing new starts — a turn begun then
would die with the process.

## Context never becomes shell syntax

Context reaches a `command` step **only through environment variables, never
interpolated into the command string**. A branch name containing a space or a
`;` becomes a variable's value, not shell syntax — that removes an entire class
of injection.

## Nothing runs that you did not set up

There is no "work a hundred iterations and tell me at the end" mode, and
nothing schedules or retries work. A step that fails leaves the card where it
is with the reason written on it.

A step that passes does what its lane says: `manual` leaves the card alone,
`ask` asks before moving it, `auto` moves it and runs the next lane's step. A
chain of `auto` lanes is one somebody built lane by lane, and the board says
which lanes those are.

That shape is deliberate. Orchestration that keeps itself going is
orchestration you cannot see, and the problem this project attacks is losing
sight of what is happening.

## An orchestrator is a chat you drive

The orchestrator is you — or a chat you drive, whose every session is on the
board. It is a project devpit keeps for itself, under `orchestrator/<account>/
<name>`, known by its folder rather than by a column, so it gets the chat, the
files and the agent API's scope the way any project does. From there, and only
from there, the agent API reaches past one project: `devpit_projects`,
`devpit_sessions`, a `project` on the board tools, and `devpit_start_session`,
which starts a background session in a card's own checkout and links it to the
card.

The limits are the point:

- **Visible.** Every session it starts is on a card, on the board.
- **No steps.** The gate that refuses an agent a lane with a step applies to it
  too; starting a step stays the person's move.
- **No consent by proxy.** A message between sessions approves nothing — the
  CLI's rule. A session waiting on the person is answered from the
  orchestrator's Sessions panel, which types into that session's own terminal as
  the person. That path is the window's alone; no agent reaches it.
- **Heard, not polling.** Its conversation keeps one process between turns, so a
  session's reply wakes it; nothing here runs it on a timer.
