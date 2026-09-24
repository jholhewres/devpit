# Changelog

What changed between releases, written for the people who use devpit. The
release notes are taken from here when a version is tagged.

## Unreleased

### Orchestrator

- **An orchestrator per Claude Code account**, at the top of the project rail.
  The first time, it makes a folder of its own under `~/.devpit/orchestrator/`
  — a git repository with a brief, `docs/`, `artifacts/` and `context/` — and
  opens a chat there; after that it picks up the last conversation. Its chat
  speaks as that account only, and it is not listed among the projects.

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
