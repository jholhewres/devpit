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
- **Agents are files** — markdown with frontmatter in `~/.devpit/agents/`. Write
  your own next to the ones that ship. Nothing to recompile, nothing to register.
- **File tree and diffs** beside the terminal, so reviewing what an agent did
  does not mean leaving.
- **Local-first.** One machine, no account, no cloud.

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

A column either does nothing, or runs one of three kinds of step:

| | `agent` | `session` | `command` |
|---|---|---|---|
| Takes the terminal? | no | **yes** — it is the target terminal | no |
| Good for | refine, review, verify | implementing | tests, builds, deploys, lint |
| Gives you back | schema-validated JSON, with cost and duration | a session you drive | streamed output and an exit code |
| How many at once | several | **one** | several |

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

### Nothing advances on its own

A step that finishes does not push the card. It writes down what happened —
output, exit code, real cost — and stops. The next move is yours.

There is no orchestrator in devpit because the orchestrator is you. That absence
is the product, not a gap in it: orchestration that keeps itself going is
orchestration you cannot see, and losing sight of the work is the problem this
attacks.

**No card, just a terminal.** Open devpit, type into it, and your agent CLI
behaves exactly as it always has. The board is an optional source of work, not a
toll gate.

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
make test     # guards, Rust tests, frontend tests
make build    # release bundle, frontend included
```

`make` on its own lists every target.

## More

- [The model](docs/the-model.md) — the seven objects everything is built from
- [Architecture](docs/architecture.md) — layout, and the rules the build enforces

## Licence

[Apache 2.0](LICENSE).
