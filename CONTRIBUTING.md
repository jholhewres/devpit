# Contributing

Thanks for looking. The project is early and moving, so the most useful thing
you can do before writing code is open an issue describing what you want to
change.

## Getting set up

See the README for system dependencies, then:

```sh
make setup
make test      # should pass on a clean clone
make dev
```

## What has to pass

```sh
make test
```

That runs the architectural guards, the Rust test suite, and the frontend
tests and build. CI runs the same target, so if it passes locally it passes there.

## The rules that have teeth

Each of these is enforced by a test that fails when the rule is broken. They
are not style preferences.

**Nothing under `crates/` may import the shell.** Product logic lives in plain
crates; `apps/desktop` adapts it to Tauri. If a crate needs the shell, the
design is wrong. `cargo xtask check` names the file and line.

**A command exists only if something calls it.** Add the command and its
caller in the same commit. A backend answering a question nobody asks is the
same defect as a button that does nothing, read from the other side.

**Responses are objects, never bare lists.** Tomorrow's extra field must not
break today's consumer, and a bare list has nowhere to put it. Arguments are
named, never positional.

**Types come from Rust.** TypeScript in `web/src/gen/` is generated. Never
edit it, and never hand-write a type that already exists on the Rust side —
two copies of a type are two copies that will drift.

**Paths are validated against the project root.** Every filesystem read is
resolved through symlinks first, then checked for containment. This process
runs terminals and writes files; reaching it is reaching the machine.

**Nothing is named after nothing.** No `utils`, `helpers`, `common`, `misc` or
`shared`. Those names say nothing about what does *not* belong in the file, so
everything ends up there. Name it after the concept it holds.

**Files only get shorter.** `xtask/ceilings.txt` caps every file over 120
lines, and the cap fails in both directions: a file over its ceiling fails, and
so does a ceiling above what the file needs. A ceiling that can be raised is
not a ceiling. Once a file is genuinely shorter, run `cargo xtask ceilings` to
write the new numbers.

## Style

Comments in English, and only where they carry a reason the code cannot. Prefer
explaining *why* over restating *what*. `cargo fmt` and the editorconfig
settle everything else.

Commit messages: a short imperative subject, and a body when the change needs
one. No trailers.

## Reporting a security issue

Do not open a public issue. See [SECURITY.md](SECURITY.md).
