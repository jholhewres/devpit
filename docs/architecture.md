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
crates/devflow   devflow.yaml, its detection, setup and evidence; flow/ ships inside it
crates/remote    the Remote through the relay: Noise, frames, the relay client, MCP between machines
crates/sync      sync between machines: the key, sealing, the clock, the merge, the namespaces
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

## A channel talks back

The Telegram chat paired with devpit is the person, and no other chat is:
anything else written to the bot is counted and never read. What the
person writes or says there becomes their turn in one orchestrator's chat —
the one chosen in Settings, or the only one — through the composer's door,
marked as sent from that channel, and the turn's last words go back at the
privacy chosen there: nothing named, names and a first line, or whole.
A voice note is fetched up to 3 MB, heard by the engine Settings set up and
deleted.

From a channel a typed yes approves nothing. Sending a draft, stopping a
session and moving a card into a lane with a step become buttons under the
answer, and the press does it, as the person. `channel_talk` takes any
transport; Telegram through the person's own bot is the first.

## Devflow is switched on by a click

Devflow — briefing, context, plan, execution, review, completion — is a
module of devpit's MCP. A project configures it in
`.devpit/devflow/devflow.yaml`, which is the source of truth: the store keeps
no copy, only that the person clicked.

- **Three levels.** Settings switches it off everywhere, and then the MCP
  lists exactly what it did before Devflow. Otherwise every project gets
  `devpit_devflow_status`, `_setup` and `_set`, which is how it is set up;
  the tools of its stages come only where it is on, stage by stage.
- **Agents write files, people switch it on.** Setup and set preview unless
  told to apply, and never commit. `enabled: true` and Decisions in
  `enforce` are never written by a tool: they wait for the person's click on
  a card under the top bar. A file that says `enabled: true` by itself — a
  hand edit, a fresh clone — waits for that click too.
- **Open sessions hear it.** The MCP server declares `listChanged` for
  tools, resources and prompts, and watches the project's devflow.yaml and
  the app's stamp file, so switching Devflow on or off reaches a session
  already running.
- **The board is the plan.** `devpit_devflow_brief` writes the brief as an
  epic card, `[devflow] Brief: <title>`. `devpit_devflow_plan` previews the
  epic's cards and creates none: the preview waits on the epic, where the
  person moves, cuts or edits it and clicks Create. An agent's `apply`
  creates only the plan the person last saw, and only once. A card's epic
  and blockers are rows (`card_parent`, `card_blocker`); its criteria stay
  in its body. An agent cannot start a card, or move it into the lane for
  work in progress, while a card blocking it is not done.
- **The card keeps its proof.** `devpit_devflow_evidence` runs the card's
  checks through the step runner, in the checkout the agent stands in, the
  card reaching them only through the environment. Each run is a folder,
  `.devpit/devflow/runs/<card>/<time>/`, git-ignored by the setup's
  `.gitignore`: each check's report and log, the screenshots and video an
  E2E check keeps, and `evidence.json` with the criteria's tests and how
  they ended. A check's verdict is a command step's — exit zero with no
  report is not a pass. The files are copied among the project's artifacts
  under `devflow/<card>/<time>/` and linked from a comment devpit signs, so
  they open on the card; the newest five runs stay in each place. With
  Playwright the setup writes `.devpit/devflow/playwright.config.ts`: the
  project's own config as its base, Devflow's specs from `e2e_dir`, a JUnit
  report and every screenshot and video.
- **A card is done at the gate.** `devpit_devflow_gate` checks what devpit
  can check itself: the card's checks ran after the last change to the code
  (each changed file by when it was written, so a commit is not a change)
  and passed, every criterion maps to tests that passed, the card's E2E spec
  kept a screenshot, and `findings.json` has nothing in `review.block_on`.
  Then, where Decisions is set up, the `devflow-done` rubric — `done` asked
  once per criterion — which in `shadow` only records and in `enforce`, the
  person's click, can hold the card; without a key the checks alone decide.
  It writes `gate.json` into the run, attaches it, and signs the comment
  `[devflow] gate: n of m criteria with evidence`. `devpit_finish_card` waits
  for it.
- **Hooks only on devpit's own launches.** A session devpit starts in a
  project where Devflow is on gets its hooks on the launch line, never in a
  CLI's settings: Claude Code a plugin folder (`--plugin-dir`, skills
  `/devflow:<stage>`), Codex the hooks with `-c` and the trust Codex keys
  them by. `Stop` and `SubagentStop` call the gate (`mcp_tool`); a failing
  one sends the turn back with what is missing. A stop the CLI did not
  continue starts the count again; past the cap (Claude Code's own, 8), or
  sent back and back with nothing changed, the turn ends and the card and a
  notice say so. A subagent is held only for code it changed. `SessionStart`
  runs `devpit agent devflow-context`, which gives the session its card.
  Switched off, the next launch has none of it, and a running session's
  gate answers nothing.
- **Skills by layer.** Where it is on, each stage's skill is a
  `skill://devflow/<name>/SKILL.md` resource carrying its SEP-2640 entry
  (digest and size), the `devpit_devflow_skill` tool, and a prompt such as
  `/devpit:devflow-plan`. The app resolves it from the project root: the
  project's `.devpit/devflow/skills/`, then the person's under devpit's home,
  then the one built from `flow/skills/`. When hosts declare
  `io.modelcontextprotocol/skills`, `skills/list` and `skills/get` join
  these with the same entries, and the tool is deprecated.

## Remote

devpit can be reached from the person's other devices over their Tailscale
tailnet, or from anywhere through devpit.app's relay. It is off unless turned
on in Settings, and one way in is open at a time.

**Through the relay** the machine connects out to devpit.app as the
person's account, and a device signed in to the same account is joined to it
there. Inside, a Noise session (`Noise_XX_25519_ChaChaPoly_BLAKE2s`) proves
the machine's key — named by the pairing QR code — to the device and the
device's key to the machine; devpit.app carries frames it holds no key to.
A device is known by its key, and forgetting it ends its session. The protocol
is `docs/remote-relay.md`.

**MCP from another machine** goes the same way and nowhere else. An MCP
client is a device the machine paired as one; `devpit mcp --remote` on the
other computer is a stdio MCP server that seals each message for this
machine, and this machine answers it from a read-only allowlist —
projects, boards, cards, sessions, what waits on the person — refusing every
other tool before the app is asked. Off unless allowed in Settings → Remote;
turning it off or forgetting the client cuts it at once. claude.ai and the
Claude app, which would have to end the encryption themselves, stay out.

**Over the tailnet** the machine serves its own viewer (`web/remote.html`):

- **Where it listens.** On loopback, published by `tailscale serve --https`.
  When the tailnet has no HTTPS, it listens on the machine's tailnet IP
  alone. It never listens on every interface; `cargo xtask check` enforces
  this with `nothing_listens_beyond_loopback`.
- **Who gets in.** A caller must be the machine's own owner in the tailnet
  (the `Tailscale-User-Login` header, or `tailscale whois`). The caller must
  also be a paired device, holding a token from a one-time code shown on
  the machine.
- **What a device may do.** Every device can watch. Typing and answering
  are granted per device on the machine.
- **Terminals.** Each watched terminal is a tmux client of its own,
  `ignore-size`, and `read-only` for a device that may not type. The desk
  never changes.
- **Secrets.** A device allowed to type can keep a secret in a project's
  `.env` through the window's own write. The pairing is read again first, a
  device keeps at most five a minute, and the value is never said back,
  logged or sent on.
- **Talking.** The orchestrators, their accounts' sessions and a project's
  chats open as a chat on the phone. A message to a chat is the person's
  turn through the composer's own `chat.send`, marked for the agent as sent
  from the Remote, and streams to the phone the way a chat reopened
  mid-turn joins it; a message to a session is typed into its terminal as
  the person.
  A photo or a recording goes in pieces under the message ceiling, at most
  8 MB and 3 MB: the photo is kept with the project's pasted pictures, the
  recording is heard by the engine Settings set up and then deleted.

The protocol is `RemoteIn`/`RemoteOut` in `devpit-rpc`, one JSON message per
WebSocket frame on the tailnet and sealed in Noise frames through the relay;
the viewer is the same either way.

## Sync

Two machines of one account share preferences and project settings through
devpit.app, sealed on each with a key devpit.app never holds. Off unless
turned on in Settings → Account → Sync. The design is `docs/sync.md`.

- **One key per account**, made on the first machine and carried to the
  next by the person: an offer the first makes on the relay, taken inside
  the Remote's Noise session, or the recovery code. A machine joining an
  account that syncs is never given a second key.
- **Sealed blobs.** Each namespace — preferences, project settings, the
  machines — is one blob, XChaCha20-Poly1305 under a key drawn from the
  account's, named by an HMAC under another, bound to its name and version.
- **Allowlists.** What syncs is listed per namespace, and a preference added
  later stays local until someone lists it; paths, windows, remote access and
  the vault never sync.
- **Field by field.** A round merges each field by a hybrid logical clock, the
  later change winning and a tie going to the same machine everywhere, so
  every machine settles on the same values in any order. A project is the
  same project on two machines when it clones the same repository.
- **The board** is the next namespace, and stays on each machine until it is
  safe to sync.
