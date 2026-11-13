# The project agent

A project agent is one Claude Code session per project that holds the
project's context: its architecture, rules, decisions, pitfalls and history.
The orchestrator asks it about the project instead of reading the project
itself, and has it write the brief a card's session starts with instead of
writing that brief by hand.

It answers and briefs. It does not edit files, move cards or start sessions,
and it stays in its own project.

## How the orchestrator reaches it

Two MCP tools, offered to devpit's orchestrator only:

| Tool | Takes | Answers |
|---|---|---|
| `devpit_project_agent_ask` | `project`, `question` | `{ project, answer, woke }` |
| `devpit_project_agent_brief` | `project`, `cardId` | `{ project, cardId, brief, woke }` |

`woke` says whether this call started the agent's process.

Neither call starts a card session, opens a terminal or touches a checkout.
The brief is text for the orchestrator to pass as `devpit_start_session`'s
`prompt`. The card's own text goes to the agent marked as data, not as
instructions.

## The process

The agent is a Claude Code process started in the project's folder, under
the profile of the orchestrator that woke it, with devpit's MCP tools. It
runs in the CLI's plan mode whatever the profile's own settings choose, so it
reads and never writes. Plan mode keeps the profile's Bash allow rules, so the
process is also started with deny rules, which win over any allow rule:
`Edit`, `Write`, `NotebookEdit`, and Bash's `rm`, `mv`, `git commit`,
`git push` and any `>` redirect.

- **Wakes when called.** The first question starts the process. Further
  questions go to the same process, one at a time.
- **Idles out.** Ten minutes after the last question with nothing new, the
  process is closed.
- **Resumes with its memory.** The next question starts a new process that
  resumes the same conversation (`--resume`), and its first message carries
  the memory summary again, read afresh from the files.

## Memory

Kept in the project's folder, in `.devpit/project-agent/`:

| File | Holds |
|---|---|
| `facts.md` | what is true of the project: guides to read, how things are built and run |
| `lessons.md` | what went wrong before and how to avoid it |
| `decisions.md` | what was decided and why, such as the project's ADRs |

Each entry is one bullet with its date and its source:

```markdown
- 2026-11-13 · AGENTS.md · A guide to read first: "Working in this repository".
- 2026-11-13 · docs/adr/0001-sync-the-persons-workspace.md · 1. Sync the person's workspace
```

The source is a file, a card (`#12`) or a pull request (`PR #3`). A bullet
without a date as `YYYY-MM-DD` or without a source is not an entry: it is
left out of what the agent is handed, and counted as left out.

The files are written on the agent's first wake, from the guides the project
already has (`AGENTS.md`, `CLAUDE.md`, `README.md`, `docs/architecture.md`)
and its ADRs in `docs/adr/`. After that, devpit never writes over them; it
only appends the lessons the person approves.

The agent is woken with a summary: each file's newest entries, at most twenty
each, and how many more there are. It reads the files themselves when it
needs the rest.

## Lessons from closed cards

When a card lands in the done column, devpit:

1. Logs one line to `metrics.jsonl`: the day, the card, its review rounds
   (one per `[devflow] review:` comment) and the findings its last review
   found again (`repeats` in its `findings.json`), so the lessons' effect can
   be measured.
2. Asks the project agent, under the profile of an orchestrator linked to the
   project, what the card teaches. It reads the card's comments (its handoff,
   its review and gate results, the person's corrections) and its last
   review's findings, as data, and answers zero to three lessons.
3. Keeps them in `proposed.md`, each dated and with the card as its source.

Each proposal waits under the top bar and on its card, with Approve, Edit
and Discard. Approved, as it is or edited, it is appended to `lessons.md`
with the card and the day. Discarded, it is removed and nothing keeps it.

The newest twenty approved lessons go into every brief the agent writes and
into every Devflow review of the project, as items of the checklist the change
is held to.

## Projects without an agent

A project gets `.devpit/project-agent/` only when its agent is first asked.
A closed card is measured and its lessons asked for only in a project that has
that folder, so a project whose agent was never asked works exactly as it did,
and every other tool keeps its shape.
