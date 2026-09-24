# Changelog

What changed between releases, written for the people who use devpit. The
release notes are taken from here when a version is tagged.

## Unreleased

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
