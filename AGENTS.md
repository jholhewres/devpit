# Working in this repository

Instructions for any coding agent or contributor. `CLAUDE.md` points here on
purpose: two copies of a rule are two copies that drift.

Read [`docs/the-model.md`](docs/the-model.md) for the domain and
[`docs/architecture.md`](docs/architecture.md) for the boundaries before making
a structural change.

## Language

**Everything versioned is in English** — code, comments, test names, commit
messages, docs, workflows. No exceptions.

`.project/` is the only place that is not, and it is gitignored.

## Commands

```sh
make test     # guards, Rust tests, frontend tests — run this before saying done
make fmt      # rustfmt + prettier; run it after any Rust edit
make dev      # the app, hot reload
make build    # release bundle
```

The Makefile has exactly seven targets: `help setup dev build test fmt clean`.
Do not add an eighth. A one-off belongs in the shell, not in the interface
everyone reads.

CI runs `setup`, `test` and `build`.

## Guards that fail the build

`cargo xtask check` enforces seven rules. Each reports file and line, because
"the guard failed" sends someone looking and a location sends someone fixing.

| Guard | Rule |
|---|---|
| `core_does_not_know_the_shell` | nothing under `crates/` imports `apps/desktop` |
| `only_one_crate_drives_the_agent` | agent CLI invocation lives in `crates/agentcli`, nowhere else |
| `nothing_is_named_after_nothing` | no enum variant, command or type that nothing draws or calls |
| `platform_window_matches_the_base` | the per-platform Tauri config cannot drift from the base |
| `files_only_get_shorter` | every file listed in `xtask/ceilings.txt` stays under its line count |
| `a_control_either_works_or_goes` | no `<button>` in `web/src` without a handler |
| `paths_come_from_home` | a path under devpit's `projects/` is built only in `crates/core/src/home.rs`; the joins onto an agent CLI's own `projects/` are counted in `xtask/cli-roots.txt` |

The ratchet only tightens. A file over its ceiling fails, and so does a ceiling
set higher than the file needs. After a file genuinely shrinks, regenerate with
`cargo xtask ceilings`.

**A control either works or leaves the screen.** `xtask/dead-controls.txt` is
now empty, so the budget is zero everywhere: a button with no handler fails the
build. A control that cannot be wired yet gets deleted, not disabled and left
looking clickable — the markup is in git and in the prototype when it becomes
real. `cargo xtask controls` regenerates the file and, like the ceilings, only
ever tightens.

## Tests

**A test must call the rule, not restate it.** A test that re-implements the
condition it is checking passes whether or not the code still applies it. When
a rule is worth testing, extract it into a named function and have the test
call that function.

Prove a test guards something by breaking the rule on purpose and watching the
test fail. If it still passes, the test is decoration.

Integration tests are named for the behaviour they pin, as a sentence:
`a_card_crosses_the_board.rs`, `an_agent_reaches_the_target_terminal.rs`,
`the_pane_shows_no_tmux_chrome.rs`.

Beware of test data with a load-bearing length. `assert_eq!(len(), 8)` next to a
literal that someone will later rename is a trap; say why the length matters or
pick a literal nobody will touch.

## Contract and types

`crates/rpc` is the contract. Frontend types in `web/src/gen/` are **generated**
— never hand-edit them, never write a second copy of a type by hand.

Responses are objects, never bare lists. Tomorrow's extra field needs somewhere
to live.

A command exists only if something calls it. Command and caller land in the
same commit.

## Security rules that are not negotiable

- Every path is resolved through symlinks and checked against the project root
  before it is read or written. This process runs terminals; reaching it is
  reaching the machine.
- Context reaches a `command` step **only through environment variables**, never
  interpolated into the command string. A branch name with a `;` in it must
  become a variable's value, not shell syntax.
- Every read of an externally supplied size has a ceiling before the allocation.

## Commits

Short, objective, English, imperative or descriptive — match what is already in
`git log`. One theme per commit.

**No trailers.** No `Co-Authored-By`, no attribution footers, no tool
signatures.

Never commit unless asked.

## Style

Match the surrounding code: its naming and its idioms.

Comments explain *why*, not *what*, and they are **short**. One or two lines.
A comment restating the line below it is noise; a paragraph explaining a
three-line function is worse, because the next reader skips both. Record the
reason a decision went one way, then stop.

Commit messages the same: a subject line that says what changed, and a body
only when there is a reason someone would otherwise have to guess at.

**Grep a CSS class name before you write it.** The shell's styles are one global
cascade of well over two thousand lines, split into `web/src/shell/styles/` and
imported in order by `shell.css` — grep the whole folder, since a short name is
a name something already has. A rule goes in the part that styles the same
thing; order across parts is the cascade. This has now shipped twice: `.act` for a transcript row met `.act`
for a sidebar button and put a border around every nav item; `.pane` for a
terminal leaf met `.pane` for the tab wrapper and set `display: none` on the
thing it was introducing. Neither is caught by anything — TypeScript does not
see CSS, and the size ratchet only counts lines. The only guard is looking
first, and a two-word name (`.arow`, `.tleaf`) when the obvious one is taken.

## Tests

Fast unit tests, and visual checks where the thing is visual. `make test`
takes seconds per change, reaches no network and exercises no service: that is
the bar every commit clears, and nothing slow belongs in it.

Two things sit outside it, deliberately. `make e2e` drives the built app
through a WebDriver and takes minutes, so it is its own target and its own CI
job — a suite nobody can run locally is a suite nobody fixes. And
`release.yml` builds, signs and publishes on a tag; it is the one job that
holds a key, and the one place GitHub is exercised.

The rule is the same as it ever was, stated as its reason rather than as a
count: the Makefile is the interface everyone reads, and CI does not invent
commands that cannot be run by hand.
