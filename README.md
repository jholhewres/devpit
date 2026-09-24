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
- **A browser pane**, for the page the project is serving. It opens
  `localhost` over http because that is what a dev server answers, keeps each
  pane's session apart, and can bring a signed-in session over from Chrome,
  Firefox or Safari — per browser and per domain, never on its own. An agent
  can drive a page you give it, and never reads what you type into a password
  field.
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

### An orchestrator, when you want one

An orchestrator is a chat you drive that sees every project at once. Made from
the top of the rail — which Claude Code account it runs as, and a name — it
lists the projects and their boards, sees the account's sessions running now,
and can hand a card's work to a new session: in the card's own checkout, linked
to the card, on the board like any other. It hears those sessions between your
messages and says what they reported in its chat.

It is still you deciding. It never moves a card into a lane that runs a step,
nothing it does runs on a timer, and a message it sends approves nothing in
another session — when one waits on you, you answer it from the orchestrator's
Sessions panel, as yourself.

**No card, just a terminal.** Open devpit, type into it, and your agent CLI
behaves exactly as it always has. The board is an optional source of work, not a
toll gate.

## Requirements

- **Linux**, X11 or Wayland, is the build. macOS has a universal build that is
  not signed or notarised (see [By hand](#by-hand)); Windows is not built yet.
- **`tmux`**. Terminals are tmux panes, which is how they outlive the window.
- **[Claude Code](https://claude.com/claude-code)** (`claude` on your PATH).
  Chat, agent steps and session steps all run through it; it is the only CLI
  devpit knows how to drive today.

## Installing

One line, on Linux or macOS, and it always takes the latest release:

```sh
curl -fsSL https://raw.githubusercontent.com/jholhewres/devpit/main/install.sh | sh
```

It picks the file that fits the machine, **refuses to install anything that
does not match the release's `SHA256SUMS`**, and puts it in place: the `.deb`
through apt on Debian and Ubuntu, the AppImage into `~/.local/bin` — with an
entry and an icon in the application menu — on other Linux, and `devpit.app`
into Applications on macOS. Read
[`install.sh`](install.sh) before piping it into a shell — it is short on
purpose. Run it again at any time to catch up; it says so when there is
nothing to do.

Three settings, all optional: `DEVPIT_VERSION=0.1.7` installs that version,
`DEVPIT_FORMAT=appimage` takes the AppImage even where apt is available, and
`DEVPIT_DRY_RUN=1` downloads and verifies without installing.

### Staying up to date

Once installed, devpit checks for a new release when it opens and once a day
after, and says so in a card — nothing to run by hand. The switch is in
Settings → General, on unless you turn it off. Your terminals keep running
through an update: they are tmux sessions, and tmux does not go down with the
window.

- **AppImage** — downloads, verifies the signature, installs over itself and
  restarts.
- **`.deb`** — downloads and verifies, then installs through polkit, which asks
  for your password: devpit never holds root itself. It restarts into the new
  version once the package is in. (Updating *to* 0.1.6 still needed closing and
  reopening devpit by hand; from 0.1.6 on it restarts itself.)
- **macOS** — downloads the `.app`, checks the signature and replaces itself.

### By hand

Every file is on the [latest release][releases], and the name carries the
version, so take the current one from that page:

- Linux: `devpit_<version>_amd64.deb` / `_arm64.deb`, or
  `devpit_<version>_amd64.AppImage` / `_aarch64.AppImage`.
- macOS: `devpit_<version>_universal.dmg`, for Apple silicon and Intel alike.

The macOS build is **not signed with an Apple Developer ID and not
notarised**, so a copy downloaded through a browser is refused by Gatekeeper
the first time with "devpit is damaged" or "cannot be opened". That is the
missing certificate talking, not the file. Open it once with right-click →
Open, or clear the quarantine flag yourself — after checking `SHA256SUMS`:

```sh
xattr -dr com.apple.quarantine /Applications/devpit.app
```

**Windows** is not built yet. The terminal is tmux and tmux is not a thing
there, so it is a port rather than a build.

Every release carries a `SHA256SUMS`, and checking a download is one line:

```sh
sha256sum -c SHA256SUMS --ignore-missing
```

The `.sig` beside each package is the updater's, not a substitute for this: it
is what an installed devpit checks before it replaces itself, against a public
key compiled into the binary you are already running. A first download has no
such binary to check it with, which is what `SHA256SUMS` is for — and what
`install.sh` checks for you.

### Uninstalling, or switching

Removing devpit never removes your work. Projects, cards, conversations and
the terminal sessions live in `~/.devpit`, which belongs to you and not to the
package — so a reinstall, in any format, opens exactly where you left off.

- **`.deb`** — `sudo apt remove devpit`
- **AppImage** — `rm ~/.local/bin/devpit ~/.local/share/applications/devpit.desktop ~/.local/share/icons/hicolor/*/apps/devpit-desktop.png`
- **macOS** — `rm -rf /Applications/devpit.app`

To erase everything, remove `~/.devpit` as well — and `~/.devpit-dev`, if you
ever ran a development build.

To switch from the `.deb` to the AppImage, remove the package and run the
installer with `DEVPIT_FORMAT=appimage`; without it, a machine that has apt is
given the `.deb` again:

```sh
sudo apt remove devpit
curl -fsSL https://raw.githubusercontent.com/jholhewres/devpit/main/install.sh | DEVPIT_FORMAT=appimage sh
```

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
