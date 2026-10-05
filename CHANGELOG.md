# Changelog

What changed between releases, written for the people who use devpit. The
release notes are taken from here when a version is tagged.

## 0.1.36 — 2026-10-05

### Chat

- **Answers are written out as they arrive.** Text appears word by word at
  a steady pace from the first token, with a caret while the turn runs, and
  says *Thinking* as soon as the agent starts to think. With reduced motion
  it still arrives as it is written, without the animation.
- **An answer to a message from another session is one paragraph again**,
  not one line per piece it was streamed in — answers already saved that
  way read whole too.
- **A reply the orchestrator drafts is a card in its chat**, with *Send*,
  *Edit* and *Drop*: you send it to the session's terminal as yours without
  leaving the chat. A session stopped on a question shows it in the same
  card. The orchestrator still sends nothing itself.

### Sessions

- **What a session has spent is shown where you use it**: in a terminal's
  corner, a chat's, each row of the Sessions panel and the island. Its
  tooltip has the last turn, what was spent since devpit first saw it, the
  tokens and the model. Dollars are API list prices; a click shows tokens
  instead.

### Fixes

- **A session in a project added a moment ago is in an orchestrator's
  reach** right after it is linked; it used to be refused for a few seconds.

## 0.1.35 — 2026-10-04

### Chat

- **Answers stream as they are written.** Text arrives word by word instead
  of all at once when the turn ends, and the line under a live turn says what
  it is at: the command or file it is on, *Thinking* or *Writing*.
- **A queued message can go now.** *Send now* on a message waiting in the
  queue hands it to the turn that is running, without waiting for it to end.
- **Links in an answer open in your browser**, `http://localhost` included.
  A link that cannot be opened says why instead of doing nothing.

### Board

- **A lane's step can start from a recipe**: *Tests* runs the command your
  project's own files name and reads its report; *Review* has an agent file
  findings on the card's change and sends a blocker back; *Prove-It* runs a
  bug's test on the card's base and on its checkout, and is not proven unless
  it failed first.
- **A review's findings sit on their lines in the card's diff**, with how bad
  each one is, why, and whether it was made against older code. A count over
  the diff goes to the first one, and a finding can be dismissed.
- **Command steps read their test reports**: a green run with passing tests
  is *Passed*, not *Inconclusive*.
- **Worktree preparation's shared variables** reach steps and terminals in a
  card's checkout.

### macOS

- **Agents are found when devpit is opened from Finder.** devpit reads your
  login shell's `PATH` and the usual install folders (the native installer,
  npm, bun, nvm, asdf, mise, Homebrew), a command in Settings may be `~/…` or
  absolute, and terminals keep UTF-8 without a locale.
- **Copy environment info**, under the agents in Settings, gives what devpit
  sees of your machine for a bug report; when no agent is found, devpit says
  where it looked.

### Fixes

- **devpit's MCP tools no longer time out after hours open.** Every question
  an agent asked listed the status of every checkout of every project; it now
  answers in a fraction of the time, and says so in time when it is slow.
- **A reloaded window ends the terminal connections its previous page left.**

## 0.1.34 — 2026-10-02

### Orchestrator

- **The Sessions panel lists what runs.** A linked project with nothing
  running no longer takes a group unless you ask for it. A project the
  orchestrator does not reach is linked from its group. A row names its card,
  stops its session from a button, and a search finds a session by name,
  project or card.

### Settings

- **Every row is a card, and only the switch switches**: clicking a row's
  words no longer turns it on. The long half of each row folds under *How it
  works*, and a search scrolls to the row it matched.
- **Remote is one card**: its steps, the address with a Copy button, and
  chips for what each device may do. A paired device is named after its
  system rather than *A device*.
- **Voice looks for a whisper again** from Settings, without reopening it.

### Fixes

- **Closing the window ends devpit**, and *Quit devpit* in the tray always
  does. devpit used to stay running behind the island with no window, and the
  tray could no longer open or quit it. Terminals keep running in tmux.
- **A session resumed after a restart is placed in its own terminal.** It
  could be taken for another pane's: its screen could not be read, and
  stopping it closed the other session's terminal.
- **Error reports leave out what is not an error**: a project that is not a
  git repository, a folder gone from the tree, a download the network cut off,
  a chat whose agent process ended, a sound device that would not start. A
  refused command is reported by its code and message, not as
  `[object Object]`.

## 0.1.33 — 2026-10-02

### Settings

- **General is grouped**: the island and being told, this desktop, chats,
  other devices, updates and privacy. Searching Settings for *island*,
  *pause*, *voice*, *remote*, *shortcut* and the rest finds it.
- **Pause has a row**, with what it holds back and what it does not.
- **The island says how it is drawn on this desktop**: above every window, a
  layer at the top, or, on GNOME, a window wherever the desktop puts it.
- **Ask before stopping a terminal** can be turned back on after *Don't ask
  again*.
- **Remote says how to get in**, step by step, what *type* and *answer*
  allow, and where its log is kept.
- **Voice messages say where the microphone is** and how to install a
  whisper.
- An orchestrator's first screen says where its rules for sessions live.

### devpit.app

- **devpit lives at devpit.app.** Sign-in and error reports go there, and the
  install line is `curl -fsSL https://devpit.app/install.sh | sh`. Installed
  copies that still ask the old address keep working.
- **The installer checks each release's signature** by devpit's key, where
  `minisign` is installed, besides its checksum.

### Fixes

- **A session that finished no longer shows as working.** Claude Code's own
  background forks report a subagent ending after the turn has ended, and that
  put the session back to work on the island, its tab and its card.
- **The microphone and the island's sound work in the AppImage.** It carries
  the GStreamer plugins it records and plays with; the system's are newer than
  its own GStreamer and were refused. The `.deb` recommends them.
- **The tray offers a pause again when one ends on its own**, rather than
  *Resume* for a pause long over.

## 0.1.32 — 2026-10-02

### Remote

- **This machine, from your phone or another computer, over Tailscale.**
  Settings → General → Remote: the machine serves its own viewer to your
  tailnet. Terminals are watched, and typed into when you allow it. The
  board's cards move. Agents' questions are answered. Chats are read.
- **Terminals are not watched from Windows yet**: a remote viewer attaches
  through a grouped session, which psmux does not have.
- **Nothing listens beyond the machine and your tailnet.** The viewer
  answers on loopback, published by `tailscale serve` with HTTPS, or on the
  machine's tailnet address when the tailnet has no HTTPS. Never on every
  interface.
- **Two things to get in.** You must be the machine's owner in the tailnet,
  and the device must be paired with a code (or QR) shown on the machine's
  screen. The code is good once, for two minutes.
- **What each device may do is set on the machine.** Every device can watch.
  Typing and answering are allowed device by device. A device that only
  watches is attached read-only by tmux itself, and a phone never shrinks
  the terminal on the desk.
- The desk says *Watched by 1 device*, and a click drops it. A log keeps who
  connected and what they did, never what was on screen.

### Orchestrator

- **A session's window says where it stands.** It shows the branch, how far
  it is ahead or behind, and whether it runs in its own checkout. A folded
  *Changes* panel holds the files, the commits and the diff. Its name is
  renamed in place (click or F2), through the CLI's own `/rename`.
- **It proposes projects, and you add them in one click.** *"Add
  ~/Workspace/x to the ASC group"* becomes a card above the composer:
  *Yes*, *Edit* or *Drop*. The Boards panel adds a folder, names it, groups
  it and links it in one screen, and links a whole group at once.
- **It hears what its sessions did since its last message** (one finished,
  one is waiting, one ended) at the start of your next one. It is not woken
  to say it.
- **Standing rules for every session it starts.** Whatever its
  `context/preferences.md` keeps under *Rules for sessions* goes with every
  brief.

### The card follows the work

- **A card moves to the lane for work in progress when its work starts, and
  to the one for checking when it is finished.** The agent no longer has to
  remember. A lane says which role it plays from its menu.
- **A card says what its sessions changed in other repositories:** *Also
  changed 13 files in devpit-app*, with which are still uncommitted.

### Chats

- **Attach files with a button** (Ctrl/Cmd+Shift+A), from the system's picker.
- **Voice messages.** Click or hold the microphone. The words land in the
  composer to read and edit before sending. They come from a whisper on this
  machine (whisper.cpp or `whisper`), or from a transcription service only if
  you choose one and keep its key. Without an engine, the recording goes as
  a file.

### The island

- **It shows the question a session asks, and answers it.** Its choices
  become buttons, pressed in that terminal as you.
- **Answer in words.** Reply to a session from the island without opening
  its terminal.
- **A question from another session waits as a badge**, rather than taking
  the island from the one you are reading. A thin bar counts down the last
  seconds before it folds.
- **On Wayland too.** It is a layer at the top of the screen on KDE,
  Hyprland and Sway (with gtk-layer-shell), and a window that never takes
  focus on GNOME.
- It costs nothing while hidden: no timer, no frame, no sound context.

### Desktop

- **One devpit at a time.** Opening it again brings the open one forward.
- **In the tray**, with *Pause for an hour*.
- **Open at login**, and **a shortcut that brings devpit forward** from
  anywhere, both in Settings → General.
- **Pause** notifications, the island and its questions, for a while or until
  you resume. Everything is still heard, so it is right when you come back.

### Other agents

- **Any agent can report to devpit.** `devpit-agent hook --agent <name>`
  says what it is doing, so it shows on the island and on its card like
  Claude Code (see `docs/agents.md`).
- **Gemini CLI reports from devpit's terminals with nothing to set up.**
  Your own `~/.gemini/settings.json` is never written, and your own hooks
  still run.
- **The groundwork for changing an agent's own settings file safely**: a
  diff first, a backup, and no write over a file that changed in between.
  Nothing uses it yet.

### Fixes

- A PDF opens in its tab, page by page, rather than as a file too long to read.
- A big tool report, such as a Read of a large file, reaches devpit cut down
  to what it reads, rather than being refused and leaving its step running.

## 0.1.31 — 2026-10-01

### Reminders

- **Ask to be reminded, and be told at that time.** A card's date can carry
  a time now; a day alone reminds that morning at nine. When it comes, a
  banner under the top bar says so — it has no close, only *15 min*, *1 hour*,
  *Tomorrow* (at nine) and *Done* — with a system
  notification when devpit is behind, and a line in the bell. A focus does
  not hold it back: the time was your own choice.
- **The orchestrator, or any session, can set one** when you ask —
  *"remind me tomorrow at three to review the PR"* — on a card with that
  date and time. devpit sets it off, not the agent, and a reminder only
  tells: it never starts anything.
- One thread waits for the next date and wakes only when a date changes.
  Switch it off in Settings → General → Reminders.

### Orchestrator

- **It drafts what you would say to a session, and you send it as yours.**
  A "go on" or a change of plan said in its chat waits, editable, above the
  composer and in the Sessions panel; *Send as you* types it into that
  session's terminal as your own words. Nothing is sent without your click.
- **A session it starts in a new folder shows the trust question** in
  *Waiting on you* before the session is even up, and is answered there.
- **Remote Control from the Sessions panel**, for a session in one of
  devpit's terminals.
- **What a session is on** — *Edit invoice.ts*, *Run npm test* — shows in
  the Sessions panel and on a card's sessions, not only on the island.
- **Every orchestrator reads the brief of the build that runs it**, however
  it is opened, and the brief says which build wrote it.

### Updates

- **The Mac updates itself.** The app in Applications was taken for a build
  nobody installs over, and was never offered an install.
- **Update in Settings shows the update it starts**: it follows the download
  and brings the update card forward, where it used to say "Working…" and
  then nothing.
- **Which devpit this is**, at the left of the status strip — with a dot when
  a newer one is on its way. A click copies it for a bug report.

### Fixes

- The island shows what is open in devpit: closing a tab takes its session
  off at once, a session idle in an open tab stays, and a terminal shows one
  session rather than every one it ever ran.
- A picture between 2 and 8 MB — a full-screen screenshot — opens in its tab
  rather than being refused as a file too long to read.

## 0.1.30 — 2026-10-01

### The island

- **Every agent at a glance, above your other windows.** A small capsule at
  the top of the screen shows what each session in devpit's terminals and
  chats is doing — the step it is on, like *Edit invoice.ts* or *Run npm
  test* — and opens all the way when one waits on you. Hover or click it for
  every session; open one for its steps and a preview of the change, the
  file or the command. It folds itself away when you move on.
- **Answer from it.** A terminal session's permission question can be
  allowed, denied, allowed always (the rule the agent suggested) or handed
  back to the terminal; unanswered, the terminal asks as it always did, and
  coming back to devpit hands it back too. A chat's question is answered
  there as well.
- **Get to the session.** *Open terminal* and *Open chat* bring devpit to the
  front on it. Drop a file on a session and its path goes into the agent's
  prompt. A session's pull request and the state of its checks show beside
  it.
- **devpit's own face**, in a picture per mood, a few sounds of its own you
  can switch off, and a system notification when a session waits, fails or
  finishes while devpit is behind — held, like the bell, by a focus.
- **Drag it** to the screen you work on; it stays there.
- On by default; switch it off in Settings → General. It sees only what runs
  inside devpit. On Linux it needs X11 or XWayland, which the AppImage uses;
  on a Wayland session without it you get the notifications alone.

### Orchestrator

- **It reads a session's replies**, a project's repository state and keeps
  its own log through devpit, rather than through scripts of its own.
- **Its links and account are kept by devpit**, out of reach of its own chat.
- **Its session tools reach only the projects linked to it**, never type
  control characters into a terminal, give every session a name of its own,
  and wait long enough for a session to start — so a slow start no longer
  ends in two of the same session.
- A message queued while it works goes in as soon as it has answered, not
  once every background task it started has finished.

### Chat

- **Allow always, for the rest of the conversation**: every file edit, the
  exact command, or the same tool — kept for that chat only.

### Fixes

- A card checked out again after its worktree was removed goes back on its
  branch, diffing from where its work began.
- A reply to a session is never typed into a terminal whose shell is in front.

## 0.1.29 — 2026-09-30

### Windows

- **Commands typed in a terminal run.** The line was cleared with keys
  PowerShell does not bind, and they reached the command as invisible
  characters — `ls` and `claude` read as not found.
- **A new terminal opens in its project's folder**, not your home — so an agent
  started there no longer asks to trust your home folder.
- **The chat finds your Claude Code profiles.** Programs are found under their
  `.exe` or `.cmd`, on the PATH Windows keeps for you now.

### Board

- **An agent moves its card with the work**: into the column for work in
  progress when it starts, and on to the one for checking when it finishes —
  by your board's own column names, never into a column that runs a step.

### Releases

- **Each release says what changed**, from this changelog: on the releases
  page and in the update card's notes.

## 0.1.28 — 2026-09-30

### Windows

- **devpit installs on Windows.** A per-user installer on the releases page,
  with psmux — the tmux devpit's terminals run on there — inside it. It adds
  devpit to the Start menu and removes cleanly from Settings → Apps. It is not
  signed with a code-signing certificate, so SmartScreen asks the first time.
- **It updates itself** like the Linux and macOS builds. On Windows an update
  closes the open terminals; the card says so before you choose.
- Terminals run PowerShell with devpit's prompt marks, find what a new Windows
  Terminal tab finds, and nothing devpit starts flashes a console window.
- Closing the last terminal ends the terminal server, and installing over
  devpit or removing it ends it first.
- The agents offered are the ones actually installed.

### Sessions

- A background session can be watched in a terminal, and every session devpit
  starts runs in one you can see (0.1.26).

## 0.1.27 — 2026-09-29

### Windows

- **devpit runs on Windows inside WSL.** The README says how: WSL 2 with
  WSLg, and the same install line as on Linux.
- **On the way to a native build.** The code now builds for Windows, and its
  terminals' layer is tested on every push against psmux, a tmux of Windows'
  own. Nothing is shipped for Windows yet.

### Fixes

- The search lists an agent by its command, without the variables in front of
  it — no more account directories or paths in the list.
- On macOS, a chat that hangs is ended after its grace period, instead of
  being taken for already gone.

## 0.1.26 — 2026-09-29

### Sessions

- **Every session devpit starts runs in a terminal you can watch.** A card's
  work handed on by an orchestrator, and a lane's `session` step, open in the
  card's terminal tab with your profile's own command — the same mode your own
  sessions run in. They used to start in the background, where nothing showed
  them and a permission they asked for waited on a question nobody could see.
- **A background session can be watched.** In the Sessions panel, "Watch it in
  a terminal" attaches it in a tab of its project.

### Fixes

- A program devpit starts — an agent, a chat, the MCP check — finds what your
  own shell finds, even when devpit was opened from the desktop's menu. MCP
  servers installed in a folder your shell adds to `PATH` no longer read as
  failed.
- The MCP list shows the servers that answer apart from the ones that do not,
  and refreshes from its header.

## 0.1.25 — 2026-09-29

### Git

- **Fetch, pull, push and sync** from the Changes panel, with how far ahead
  and behind the branch is. Pull only fast-forwards: a branch that went two
  ways is left for a terminal, never half-merged by a button.
- **The panels follow a commit made anywhere** — in a terminal, by an agent —
  instead of waiting for the window to lose and regain focus.
- **The branch menu scrolls**, and filters once there are more than a few.
- **A deleted file shows its diff** again.

### Terminals

- **A link in a terminal opens in the browser.**
- **A split stays where its pane was** — in a card's worktree, with its branch
  shown at once.
- **Stopping a terminal ends everything it started**, background jobs
  included; removing a project ends its terminals.
- **Drop files** on a terminal to type their paths, on the file tree to copy
  them in, or on Artifacts to keep them.
- **A new project offers its agents** on the empty screen.

### Orchestrator

- **A stopped session stays stopped.** A background session is stopped by its
  own CLI, which would otherwise start it again, and every session of that
  name goes with it.
- **The chat keeps its turns apart**: a turn another session woke and the
  person's own no longer mix, land out of order or stop each other, and a chat
  with Remote Control on is no longer closed by another.
- **Orchestrators stay where they are** in the rail when you pick one.

### The window

- **The right panel reads as one**: the tab in front says its name, Sessions
  counts what is waiting on you, and every view has the same heading and the
  same empty state. Artifacts can be added by hand.
- **A notice goes to the terminal** the agent is waiting in.
- **The status strip shows the MCP servers** — how many answer, and why the
  others do not.
- **A group of projects can show only the active ones.**
- **A new worktree can be set up for you**: files copied or linked from the
  main checkout, commands run, variables set — in Edit project.
- **An update shows as soon as it is found**, and devpit asks every hour.
- Less redrawing: the window no longer re-renders itself for every agent
  report.

### Fixes

- "database is locked" with many terminals open.
- The board's Add card no longer sits under a long lane.
- A card's branch keeps its accented letters (`producao`, not `produ-o`).
- Sessions under a path with `_` are found, and a profile switched off leaves
  every menu that offers one (thanks, @aronpc).

## 0.1.24 — 2026-09-26

### Orchestrator

- **Stop a session.** `devpit_stop_session` — and Stop in the Sessions panel —
  ends a session and closes the terminal it ran in, its tab too when it was the
  last pane. Only when you ask: work in flight is lost.
- **Start a session anywhere you linked.** Asked to, the orchestrator starts
  Claude Code in a new terminal tab of any linked project, in its folder — no
  card and no worktree needed.
- **New terminals and chats from the Sessions panel.** Every linked project is
  listed there, with or without a session, and opens a new terminal right over
  the chat or a new chat in the project.
- **A terminal you can size.** The terminal opened over the chat resizes from
  its edges and corners, stays centred and remembers its size; it is drawn as
  one surface in the terminal's colour. Hiding it keeps it running; "Close
  terminal" ends it.

### Performance

- Idle MCP App hosts are let go after ten minutes instead of staying until
  devpit quits.
- Listing sessions no longer asks git about every project every few seconds,
  and does nothing while the window is hidden.

## 0.1.23 — 2026-09-25

### Orchestrator

- **Sessions in one panel.** The right panel opens on Sessions: grouped by
  project, the ones waiting on you first, each with how long it has been at
  it and what it was last asked. A session opens to its question, a reply
  typed into its terminal as you, and everything that passed between it and
  the orchestrator; sessions no longer running stay below with their history.
  The chat's corner counts them and opens the panel.
- **A session's terminal, right here.** Open any session in a devpit terminal
  over the orchestrator's chat and work in it as you — answer what it asks,
  type a command — without leaving; its tab takes it back when you close it.
  "Go there" now opens its very tab.
- **Questions above the composer.** A session stopped on a question shows
  right above where you type, with its choices and a way into its terminal.
- **Who woke it.** A turn another session woke says which one, and quotes what
  it said.
- **Handed work as cards.** Each message the orchestrator sends shows as a
  card: to whom, what was asked, and how that session stands now.
- **Memory and tools, always.** The orchestrator's brief now has it search and
  save in whatever memory tools the account has, look facts up with the
  account's MCP servers, and keep a notice that adds nothing to one line.

### Chat

- **Remote Control connects visibly.** A chat whose process started with a
  message now shows its claude.ai page as soon as it arrives, instead of
  staying on "connects with the next message".

### Under the hood

- Tests no longer leave tmux servers running behind them.

## 0.1.22 — 2026-09-25

### Orchestrator

- **Projects are linked by hand.** An orchestrator no longer reaches every
  project. Link the ones it works with from its Boards panel: only those show
  there, only their folders are open to its chat, and only they are in reach
  of its tools. An orchestrator made before starts with none linked.
- **Only its own account's sessions.** The Sessions panel is back to the
  sessions of the account the orchestrator speaks as; 0.1.19 listed every
  account's.

## 0.1.21 — 2026-09-25

### Chat

- **Pages that come with MCP tools.** Tools that bring an interactive page —
  a Jira issue, a Confluence page, a Datadog chart — show it in the chat,
  under the answer that called them, as they do on claude.ai. Each page runs
  on its own origin, sandboxed, reaching only the hosts its server declared
  and nothing of devpit's. When a page wants to run one of its server's
  tools, you are asked first: once, or always for that page.

### devpit's own tools

- **The board, a card and the sessions as pages.** In any host that shows
  MCP Apps, devpit's board opens its cards, a card can be commented on, moved
  and handed to a new session (in its own checkout or the project's folder),
  and the sessions show who is busy, idle or waiting on you. A waiting
  question is still answered only in devpit.

## 0.1.20 — 2026-09-25

### Orchestrator

- **Every board beside the chat.** An orchestrator no longer has a board of
  its own. Its right panel opens on Boards: each project with its lanes and
  how many cards each holds, a lane opening to its cards, and a card opening
  on its project's board.

### Projects

- **Artifacts.** Files that belong to a project but not in its repository —
  specs, exports, reports — are kept in its own folder in the devpit
  workspace, safe from clones, new worktrees and cleans, and never committed.
  The right panel's Artifacts tab lists, opens and removes them; sessions
  keep and take them back with devpit's `devpit_artifact_*` tools.

## 0.1.19 — 2026-09-25

### Orchestrator

- **Waiting sessions are found again.** 0.1.18 did not recognise the
  terminals real Claude Code sessions run in, so "Waiting on you" and Reply
  never showed for them. They do now.
- **Sessions of every account.** The Sessions panel lists the Claude Code
  sessions of all your accounts, each marked with the account it runs under.
  Those of another account can be read and answered from the panel; only the
  orchestrator's own account can be messaged.
- **Notes, organised, without git.** An orchestrator reads every project's
  folder, keeps what it learns in `context/projects/`, your preferences in
  `context/preferences.md` and decisions in `decisions/`. Its folder is no
  longer a git repository, and the Changes and History tabs are gone for it.
- **Asks before starting a session.** Work goes to the session already
  running in a project; a new one is started only after asking, in the card's
  own checkout or the project's folder, as you choose.
- **Easier to add and tell apart.** A new orchestrator is one click away in
  the rail, and each one gets a colour of its own.

### Chat

- **Remote Control in any chat.** A project's chat can be reached from
  claude.ai and the Claude app too — outside the Supervised mode, which runs a
  process per turn.
- **Links you can click.** Web addresses and file paths in an answer are
  links, and web addresses in a terminal open in the browser. A file an answer
  names opens beside the chat, and expands into a tab.

## 0.1.18 — 2026-09-25

### Orchestrator

- **Answer a waiting session from the orchestrator.** A session in one of
  devpit's terminals that stops on a question — a permission, a choice Claude
  asked for — shows at the top of the Sessions panel beside the chat, under
  "Waiting on you", with its question and choices. A click answers it in that
  session's own terminal, as you; Esc dismisses it. Nothing is pressed unless
  the screen still shows exactly the question you clicked on, so a late or
  double click never answers the next one. The orchestrator can read the
  question and recommend a choice, but only you answer it.
- **Remote Control in the orchestrator's own chat.** The phone button on its
  chat now makes this same conversation — this chat, this process — reachable
  from claude.ai and the Claude app as `devpit-<name>`, and opens its page from
  the chat. What you write there arrives in this chat as it happens, marked as
  not from here. It replaces 0.1.17's terminal that resumed the conversation
  beside the chat. If the chat's process is not running yet, it connects with
  the next message; a restarted process connects again on its own.

## 0.1.17 — 2026-09-25

### Orchestrator

- **Continue an orchestrator remotely.** A phone button on its chat continues
  the same conversation in a terminal with Remote Control on, named
  `devpit-<name>` in claude.ai and the Claude app — hooks and devpit's tools
  included. Close the terminal and write in the chat to carry on at the desk.

### Fixes

- A choice in a dialog or in Providers is a form field whose list opens under
  it, instead of the chat's chip menu floating over the dialog's buttons.

## 0.1.16 — 2026-09-24

### Fixes

- **No one machine's commands are built in.** A second account used to be
  looked for by one person's command name; now an account is only ever a
  command its owner declares in Providers, and the process it runs is
  recognised as the CLI it is.
- **The default agent is one menu** in Providers, not a row of buttons that grew
  with every profile.
- **An orchestrator runs as the account you pick.** The picker drew no
  selection, so one could start as an account nobody meant. It is now a menu of
  the accounts configured in Providers, with a way to add another. The account
  lives in the orchestrator's own folder and **Runs as…** changes it later.
- **Orchestrators are a group of their own** — Orchestrators, at the top of the
  rail, folding like any group, with a quiet + — and edit like a project: name,
  icon and colour, with a mark of their own by default. The project picker no
  longer lists them.
- **An orchestrator opens with its history begun**: its folder is committed when
  it is made, and devpit's own files are kept out of it, instead of opening on
  a list of untracked files. *(Since 0.1.19 the folder is not a git
  repository.)*
- The project picker says "1 worktree", not "1 worktrees".

## 0.1.15 — 2026-09-24

### Orchestrator

- **Orchestrators**, at the top of the project rail. **New orchestrator** asks
  which account it runs as — by the command that starts it, a shell function
  included — and a name; an account can have several. Each gets a folder of
  its own under `~/.devpit/orchestrator/<account>/<name>/` — a git repository
  with a brief, `docs/`, `artifacts/` and `context/` — and a chat that opens on
  its last conversation. Right-click reveals its folder or removes it; the
  folder is kept. It is not listed among the projects. *(Since 0.1.16 the
  folder is `~/.devpit/orchestrator/<name>/`, and since 0.1.19 it is not a git
  repository.)*
- **An orchestrator keeps listening between your messages.** Its process
  stays, so a session's reply — or the notice it asked for when one went idle
  — wakes it, and what it says appears in its chat, marked as not in answer to
  you. Stop interrupts the turn and leaves it listening. It opens in Accept
  edits; in the supervised mode it cannot listen.
- **An orchestrator hands work to sessions.** Given a card, it starts a new
  Claude Code session of its account in the background, in the card's own
  checkout, linked to the card — on the board like any other — and named, so
  it can message it and hear when it is done. Only an orchestrator can, and
  only on a card.
- **Answer a session from the orchestrator.** A session in a devpit terminal
  that is waiting on you can be answered from the Sessions panel: what you
  type goes into its terminal as yours. A message from the orchestrator
  approves nothing there — that is Claude Code's rule, and it stays.
- **It sees every project and every session of its account.** Its agent can
  list the projects and their lanes, work on any project's board, and see the
  Claude Code sessions running now — by the name it messages them with, busy or
  idle, and the project and card each works in. The same list sits beside its
  chat. It still never moves a card into a lane that runs a step.
- **Its brief keeps up with devpit.** devpit's instructions live in
  `.devpit/orchestrator.md` and are rewritten when devpit updates; `CLAUDE.md`
  imports them and is yours alone.
- An agent started in a card's checkout reaches its own board again; the
  board's tools used to answer that no project contained it.

### Chat

- **A chat you leave keeps going.** Switch project or reload the window in the
  middle of an answer and, coming back, the answer so far is there and the
  rest keeps arriving — Stop included. It no longer says the app closed about
  a turn that was still running.
- **The mode you pick stays picked.** Full access, Accept edits or Supervised
  survives switching tabs, reopening the conversation and restarting devpit,
  and a new conversation starts in the last mode picked on that account.
- **Unsent text is kept** per conversation until you send or clear it.
- **The thread follows the answer as it grows** — text streaming into place,
  a block opening — and stops when you scroll up to read, with a button back
  to the latest.
- **A wider thread**: one 896px column for messages, the composer and replies.
- A relative link in a card's chat opens the file in the card's own checkout.
- **A chat has the board's tools**, as a terminal does, without the Claude
  Code plugin installed.
- **Sessions devpit starts hear the account's other sessions** whatever mode
  either runs in, instead of holding each message until approved by hand.

### Highlights

- **Error reports, opt-in.** A new switch in Settings → General, off unless you
  turn it on. With it on, devpit keeps its own errors — panics, internal
  errors of its commands, background routines that fail, and errors the
  window did not catch — and sends them anonymously so bugs can be found and
  fixed:
  - what is kept is cleaned as it is written: no paths, no credentials or
    tokens, no names quoted by git or the shell, no text from your cards,
    prompts or terminals;
  - a report carries only the error and devpit's version, OS, architecture and
    package type — no account, even when signed in, and no install id;
  - reports go out only while devpit sits idle (nothing running, nobody at the
    window for five minutes), five at a time, and pause when you come back;
  - Settings shows the exact request the next report would send;
  - turning the switch off deletes everything kept, at once. Kept errors never
    grow past 200 entries or 256 KB and are dropped after fifteen days, on the
    server too.
