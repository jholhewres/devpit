# Changelog

What changed between releases, written for the people who use devpit. The
release notes are taken from here when a version is tagged.

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
  a list of untracked files.
- The project picker says "1 worktree", not "1 worktrees".

## 0.1.15 — 2026-09-24

### Orchestrator

- **Orchestrators**, at the top of the project rail. **New orchestrator** asks
  which account it runs as — by the command that starts it, a shell function
  included — and a name; an account can have several. Each gets a folder of
  its own under `~/.devpit/orchestrator/<account>/<name>/` — a git repository
  with a brief, `docs/`, `artifacts/` and `context/` — and a chat that opens on
  its last conversation. Right-click reveals its folder or removes it; the
  folder is kept. It is not listed among the projects.
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
