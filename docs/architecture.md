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
