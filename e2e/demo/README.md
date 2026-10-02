# Launch assets

Screenshots and short clips of devpit for the site and the launch, made from
nothing on every run:

```sh
make shots                 # builds the release binary, then the line below
node e2e/demo/shots.mjs    # with a build already there
```

Everything lands in `target/launch-assets/`. A run takes about fifteen minutes.

## What it needs

What `make e2e` needs (`tauri-driver`, `WebKitWebDriver`, `Xvfb`, `dbus-daemon`),
plus `ffmpeg`, `xcompmgr` and `xdotool`. The script says which one is missing
and the command that installs it.

It never draws on your screen: the app runs on its own Xvfb display at 2x,
with a compositor so the island's corners are transparent.

## Nothing real in it

The world is `world.mjs`: an invented company, its four repositories, their
cards and a person called Sam Rivera who does not exist. The home is made
under `target/demo-home` (`home.mjs`), the account origin points at a closed
port, and `claude` is `agent.mjs` — a scripted stand-in that answers offline
and costs nothing. No path, name, address or token of the machine it runs on
reaches a picture.

## What comes out

Every still exists in `dark` and `light`, and in two sizes: `hero` is
1600×1000 for the site, `ph` is 1270×760 for Product Hunt's gallery — both at
2x, so 3200×2000 and 2540×1520 pixels.

| File | Shows |
|---|---|
| `board-*` | A project's board: lanes, cards with their review cost, agents working on cards, one waiting on a question |
| `island-*` | The board with the island open: every session, which one waits, what each is doing |
| `orchestrator-*` | The orchestrator's chat after planning the week, the Sessions panel, a question under *Waiting on you* |
| `session-terminal-*` | A session's own terminal, opened over the orchestrator |
| `card-diff-*` | An open card: its run, its cost, its branch and the diff of what changed |
| `browser-*` | The browser pane on a local page |
| `manager-*` | Every project's cards in one board |
| `reminder-*` | A reminder going off, on the board |
| `orchestrator-starts-session.{mp4,gif}` | Asking the orchestrator to start a session; it shows up in Sessions |
| `card-runs-its-step.{mp4,gif}` | A card moved into the review lane runs its step and gets a cost |
| `answer-from-the-inbox.{mp4,gif}` | A session's question answered from the orchestrator's chat |
| `reminder-fires.{mp4,gif}` | A reminder's banner arriving |
| `thumbnail-240.{png,gif}` | The mark, and the island's moods in a loop |

Clips are at most about 12 seconds: a scene that takes longer is played
faster rather than cut. MP4s are 1600 wide, GIFs 960.

The scenes come in two phases — what needs no agent running, then what does —
and a phase that fails is run again from a fresh home, up to three times.
When the last try fails, `failure.png` is what the screen showed.

## Why it is shaped this way

- **ffmpeg reads the screen**, not WebDriver: this driver gives no frame (see
  `lib/screen.mjs`), and the island and the browser page are native windows a
  WebDriver screenshot would leave out anyway.
- **devpit's root is a short link** into the home: a tmux socket path is
  capped near 108 bytes, and a checkout under a worktree is already past it.
- **The window is never reloaded**: now and then a reloaded page never hears
  from the app again. The seed is shown by opening a fresh window instead;
  projects are picked on the rail and the theme in Settings.
- **A card is moved and a reminder set before any agent starts**: with agents
  running, a card move into a lane with a step answers only much later.
- **Retries** exist because, on a loaded machine, the app has been seen to
  stop answering a page and once to die (`malloc(): unaligned tcache chunk
  detected`). Both are the app's to fix, and worth finding.
- **Every recorder is stopped**, on failure and on Ctrl-C: one left running
  holds a few cores for good, and every run after it starts to time out.
