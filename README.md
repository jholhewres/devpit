# quockpit

A desktop workspace for working with coding agents.

Terminals that outlive the window, a git worktree per line of work, and a
**per-project board where moving a card runs real work** — in the terminal you
already have open, not in a new one.

Rust, a Tauri shell, a React frontend. Local-first: one machine, no account, no
cloud.

> **Status: early.** The foundation — pty, git, tmux, typed contract, window —
> stands. The board and the workflow are what is being written.

## The idea in one screen

```
    [inbox]        [refine]       [review]        [doing]        [check]       [ship]
       │              │              │              │              │              │
   you write        agent          agent       agent session      agent        your own
   the card        headless       headless    in the target    headless       command
                                                 terminal                   (tests, deploy)
       └──────────── steps that do NOT take the terminal ──────┴──────────────────┘
```

You write the card. Move it to *refine* — an agent reads it and returns
questions, risks and acceptance criteria, with what that call cost. Move it to
*review* — another agent approves it or sends it back, and says why. Move it to
*doing* — now a session is born, in its own worktree, and it shows up **in the
terminal you were already using**.

Columns are yours. Rename them, reorder them, add your own, decide which one
runs what. The flow above is just the default board for a new project.

## The two rules that define the product

**One target terminal per project.** Not five live panes competing for
attention. One. Switching lines of work switches what is attached to it; the
previous session keeps running in the background, it just stops taking up the
screen.

**No card, just a terminal.** Open quockpit, type into it, and your agent CLI
behaves exactly as it always has. The board is an optional source of work, not a
toll gate.

## Three kinds of step

A column either does nothing, or runs one of three things:

| | `agent` | `session` | `command` |
|---|---|---|---|
| Takes the terminal? | no | **yes** — it is the target terminal | no |
| Good for | refine, review, verify | implementing | **tests, builds, deploys**, lint |
| Gives you back | schema-validated JSON, with cost and duration | a session you drive | streamed output, and an exit code |
| How many at once | several | **one** | several |

Every headless step declares a spending cap and reports what it actually cost. A
card shows what has been spent on it. "The agent is doing something" is not an
acceptable answer.

Context reaches a `command` step **only through environment variables, never
interpolated into the command string**. A branch name containing a space or a
`;` becomes a variable's value, not shell syntax — that removes an entire class
of injection.

## Agents are files

Agents live in `~/.quockpit/agents/` as markdown with frontmatter. Ship a set to
start from, then write your own next to them. A custom agent is a new file in
that directory — nothing to recompile, nothing to register.

## Nothing runs on its own

There is no "work a hundred iterations and tell me at the end" mode. If a step
fails, the card stays where it is with the reason written on it, and you decide
what happens next.

That absence is deliberate. Orchestration that keeps itself going is
orchestration you cannot see, and the problem this project attacks is losing
sight of what is happening.

## Requirements

- Rust 1.93+, Node 22+, pnpm
- `tmux`
- the agent CLI you use
- on Linux, the WebKitGTK development packages:

```sh
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
                 build-essential curl file libssl-dev libayatana-appindicator3-dev
```

## Running it

```sh
make setup    # dependencies
make dev      # the app, with hot reload
make test     # what CI runs: guards, Rust, frontend
make build    # release bundle, frontend included
```

`make` on its own lists every target.

## Layout

```
crates/core     domain, SQLite store, event bus
crates/git      status, worktrees, clone, log
crates/pty      pty handling, ring buffer, coalescing
crates/rpc      the contract; frontend types are generated from it
crates/tmux     the sessions that outlive the window
apps/desktop    the shell — the only crate allowed to know it
web             React + Vite + xterm.js
xtask           the architectural guards
```

## The rules that have teeth

Each one is enforced by a test that fails when the rule is broken. They are not
style preferences.

- **Nothing under `crates/` imports the shell.** If a crate needs it, the design
  is wrong. `cargo xtask check` names the file and the line.
- **A command exists only if something calls it.** Command and caller land in
  the same commit.
- **Responses are objects, never bare lists.** Tomorrow's extra field needs
  somewhere to live.
- **Types come from Rust.** `web/src/gen/` is generated; two hand-written copies
  of a type are two copies that will drift.
- **Every path is resolved through symlinks and checked against the project
  root.** This process runs terminals and writes files; reaching it is reaching
  the machine.

## Licence

[Apache 2.0](LICENSE).

This program holds credentials and runs commands on your machines. Software in
that category has to be auditable to be worth installing, and an explicit patent
grant means a company's legal review does not have to be an argument.
