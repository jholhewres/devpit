<p align="center">
  <img src="web/src/assets/brand/mark.png" alt="" width="128">
</p>

<h1 align="center">devpit</h1>

<p align="center">
  A native app for controlling your coding agents.
</p>

<p align="center">
  <a href="https://github.com/jholhewres/devpit/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/jholhewres/devpit/ci.yml?branch=main&style=flat-square&label=ci" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/licence-Apache%202.0-blue?style=flat-square" alt="Licence: Apache 2.0"></a>
  <img src="https://img.shields.io/badge/rust-1.93+-dea584?style=flat-square&logo=rust&logoColor=white" alt="Rust 1.93+">
  <img src="https://img.shields.io/badge/status-early-e2795b?style=flat-square" alt="Status: early">
</p>

<p align="center">
  <b>English</b> &middot; <a href="README.pt-BR.md">Português</a>
</p>

---

## What it is for

Agents are easy to start and hard to keep track of. Five terminals in, you no
longer know what is running, what it cost, or which one you were reading.

devpit exists to keep **every step of the work visible and under your hand** —
one terminal you are actually watching, a board that says where each piece of
work stands, and a number on every agent call.

## What it does

- **One target terminal per project.** Switching work switches what is attached
  to it. The previous session keeps running in the background; it just stops
  taking up the screen.
- **Sessions that outlive the window.** Close devpit, reopen it, and what was
  running is still running.
- **A git worktree per line of work**, made when a step needs a checkout and
  removed when you say so — never while it still holds uncommitted work.
- **A board per project.** Moving a card runs real work — in the terminal you
  already have open, not in a new one.
- **Your columns.** Rename them, reorder them, add your own, decide which one
  runs what.
- **Each column composes its own step** — which agent, on which model, with
  which slice of the card's context.
- **A spending cap on every agent call**, and the real cost written on the card
  when it finishes.
- **Agents are files** — markdown with frontmatter in `~/.devpit/agents/`, and
  whatever your tooling already installed. None ship with devpit: the ones you
  see are the ones on your machine. Nothing to recompile, nothing to register.
- **File tree and diffs** beside the terminal, so reviewing what an agent did
  does not mean leaving.
- **On your machine.** Projects, boards, conversations and terminal history
  live in `~/.devpit` on this computer. An account is optional and, today,
  signs you in and nothing else — syncing between machines is not built.

## How the board works

```
      [inbox]       [refine]       [review]        [doing]        [check]        [ship]
         │              │              │              │              │              │
     you write       planner        critic        executor       verifier       your own
     the card        · opus         · opus        · sonnet       · sonnet        command
         —            agent          agent         session         agent         command
                        └──────────────┘                             └──────────────┘
                          no terminal                                  no terminal
                                                      ▲
                                           the one target terminal
```

**That is a board somebody set up, not the one you get.** A new project gets
those six columns with nothing behind them: every lane runs nothing until you
give it a step, which is one menu on the lane.

A column either does nothing, or runs one of three kinds of step:

| | `agent` | `session` | `command` |
|---|---|---|---|
| Takes the terminal? | no | **yes** — it is the target terminal | no |
| Good for | refine, review, verify | implementing | tests, builds, deploys, lint |
| Gives you back | schema-validated JSON, with cost and duration | a session you drive | streamed output and an exit code |
| How many at once | several | several &mdash; one of them attached | several |

The flow above is just the default board for a new project.

### A column composes the step

A step is not "call an agent". It is a recipe the column holds, and moving a
card resolves it into one invocation:

| | |
|---|---|
| `agent` | who does it — `architect`, `executor`, `reviewer` |
| `model` | which model that call runs on |
| `inject` | which of the card's context reaches it |
| `capUsd` | the ceiling it may spend |
| `expects` | the shape the answer has to satisfy |

Agents come from your machine — the ones you wrote in `~/.devpit/agents/`, and
the ones your tooling already installed. devpit **references them, never copies
them**: whatever owns a catalogue keeps owning it.

The point is scope. Twenty agents available everywhere end up loaded
everywhere, and you pay for all of it on every call. A column says *this* step
is that one agent, on that model, with that slice of the card — and that is all
that gets sent.

Skills are not on that list. The CLI offers a turn all of them or none, so a
step asks for one in its prompt — `/tdd` — the way you would in a session, and
a step that tries to declare them is refused when it is saved.

### A lane moves a card only as far as you let it

A step that finishes writes down what happened — output, exit code, real cost.
What happens next is the lane's setting, and you choose it per lane:

| | |
|---|---|
| `manual` | nothing moves. The card stays where it is until you drag it. |
| `ask` | devpit asks, naming the lane it would move the card to. |
| `auto` | it moves, and the next lane's step runs. |

`manual` is the default, and a lane set to `auto` says so on the board. There
is no orchestrator behind this: nothing schedules work, nothing retries, and a
chain of `auto` lanes is one you built lane by lane and can see.

**No card, just a terminal.** Open devpit, type into it, and your agent CLI
behaves exactly as it always has. The board is an optional source of work, not a
toll gate.

## Requirements

- **Linux**, X11 or Wayland. macOS and Windows are not built yet.
- **`tmux`**. Terminals are tmux panes, which is how they outlive the window.
- **[Claude Code](https://claude.com/claude-code)** (`claude` on your PATH).
  Chat, agent steps and session steps all run through it; it is the only CLI
  devpit knows how to drive today.

## Installing

Linux and macOS are on the [latest release][releases]. On Linux, take the
AppImage unless you have a reason not to: it is the one that updates itself.

The commands below name `0.1.4` because the version is part of the file name.
Check the releases page for the current one, or let an installed devpit update
itself and never type a version again.

**AppImage (Linux).** devpit checks for a new version, verifies its signature,
installs it over itself and restarts. Your terminals keep running through it —
they are tmux sessions, and tmux does not go down with the window.

```sh
curl -LO https://github.com/jholhewres/devpit/releases/latest/download/devpit_0.1.4_amd64.AppImage
chmod +x devpit_0.1.4_amd64.AppImage
./devpit_0.1.4_amd64.AppImage
```

**`.deb` (Debian, Ubuntu).** devpit downloads the new version and verifies it,
then shows you the command and never runs it. Installing a system package means
asking for root, and devpit does not ask for root on your behalf.

```sh
curl -LO https://github.com/jholhewres/devpit/releases/latest/download/devpit_0.1.4_amd64.deb
sudo apt install ./devpit_0.1.4_amd64.deb
```

**`.dmg` (macOS).** Apple silicon only — an M1 or later. There is no Intel
build yet: the runner that makes one is the last Intel image GitHub has and it
is on its way out, so the answer for an Intel Mac is a universal binary, which
is not built yet. devpit updates itself here: it downloads the `.app`, checks
the signature and replaces itself.

The download is **not signed with an Apple Developer ID and not notarised**, so
the first open is refused by Gatekeeper with "devpit is damaged" or "cannot be
opened". That is the missing certificate talking, not the file. Open it once
with right-click → Open, or clear the quarantine flag yourself:

```sh
xattr -dr com.apple.quarantine /Applications/devpit.app
```

Check `SHA256SUMS` first if you are going to do that — see below.

**Windows** is not built yet. The terminal is tmux and tmux is not a thing
there, so it is a port rather than a build; it is not in this release.

Every release carries a `SHA256SUMS`, and checking it is one line:

```sh
curl -LO https://github.com/jholhewres/devpit/releases/latest/download/SHA256SUMS
sha256sum -c SHA256SUMS --ignore-missing
```

The `.sig` beside each package is the updater's, not a substitute for this: it
is what an installed devpit checks before it replaces itself, against a public
key compiled into the binary you are already running. A first download has no
such binary to check it with, which is what `SHA256SUMS` is for.

Automatic checking is a switch in Settings → General, and it is on unless you
turn it off.

[releases]: https://github.com/jholhewres/devpit/releases/latest

## Building it

- Rust 1.93+, Node 22+, pnpm
- the WebKitGTK development packages:

```sh
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
                 build-essential curl file libssl-dev libayatana-appindicator3-dev
```

## Running it

```sh
make setup    # dependencies
make dev      # the app, with hot reload
make test     # guards, Rust tests, frontend tests
make build    # release bundle, frontend included
```

`make` on its own lists every target.

## More

- [The model](docs/the-model.md) — the seven objects everything is built from
- [Architecture](docs/architecture.md) — layout, and the rules the build enforces

## Licence

[Apache 2.0](LICENSE).
