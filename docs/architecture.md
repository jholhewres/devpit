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
  newer one. After a restart it is empty; the process table marks a card pane
  with an agent in front as open until the agent says more.
- **The window hears it on `card:happening`.** One event per change, with the
  card's sessions and what they add up to. The tile's dot and the open card's
  Sessions section read it.
- **A conversation runs where its session ran.** Its folder is fixed when it
  begins or takes a session in, and no turn makes a checkout.

## Context never becomes shell syntax

Context reaches a `command` step **only through environment variables, never
interpolated into the command string**. A branch name containing a space or a
`;` becomes a variable's value, not shell syntax — that removes an entire class
of injection.

## Nothing runs on its own

There is no "work a hundred iterations and tell me at the end" mode. If a step
fails, the card stays where it is with the reason written on it, and you decide
what happens next.

That absence is deliberate. Orchestration that keeps itself going is
orchestration you cannot see, and the problem this project attacks is losing
sight of what is happening.
