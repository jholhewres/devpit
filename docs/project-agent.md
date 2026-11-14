# The project agent

A project agent is one Claude Code session per project that holds the
project's context: its architecture, rules, decisions, pitfalls and history.
The orchestrator asks it about the project instead of reading the project
itself, and has it write the brief a card's session starts with instead of
writing that brief by hand.

It answers and briefs. It does not edit files, move cards or start sessions,
and it stays in its own project. The project's own sessions consult it too,
while they work.

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

## How a project's sessions reach it

A session working in a project, on a card or in the project's folder, has
two MCP tools of its own, apart from the orchestrator's and the assistant's:

| Tool | Takes | Answers |
|---|---|---|
| `devpit_project_agent_consult` | `question` | `{ project, answer, woke, left }` |
| `devpit_project_agent_propose` | `kind` (`fact`, `lesson` or `decision`), `text`, `cardId` | `{ project, proposed, source, waiting }` |

Neither takes a `project`: a session reaches its own project's agent, the
same process the orchestrator asks, and no other. Both are refused to an
orchestrator, which has its own pair, and in a project whose agent was never
asked, which has none yet. The agent is woken under the profile of an
orchestrator linked to the project, as for a closed card.

- **A budget of questions.** A session asks at most ten questions; `left`
  says how many remain, and one past the budget is refused without reaching
  the agent. A question the agent could not answer is not counted.
- **Proposals, never writes.** What a session wants remembered goes to
  `proposed.md`, with the card it came from (`cardId`, or the card whose
  checkout the session stands in) as its source, and waits for Approve, Edit
  or Discard like any other proposal. The memory files change only once the
  person approves it.
- **Briefed to ask first.** A session the orchestrator starts in a project
  with an agent finds a short section after its task, under
  `## The project agent`: consult the agent before exploring broadly or
  choosing a pattern its memory may settle, and propose rather than edit
  `.devpit/project-agent/`. A brief for a project without an agent goes as it
  was.

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

The source lists, comma-separated, the files the entry rests on, the cards
it came from (`#12`) and pull requests (`PR #3`):
`- 2026-11-14 · #12, docs/adr/0002-one-queue.md · Proposals share one queue.`
An entry written before sources listed files still rests on its source when
that is a file. A bullet without a date as `YYYY-MM-DD` or without a source is
not an entry: it is left out of what the agent is handed, and counted as left
out.

The files are written on the agent's first wake, from the guides the project
already has (`AGENTS.md`, `CLAUDE.md`, `README.md`, `docs/architecture.md`)
and its ADRs in `docs/adr/`. After that, devpit changes them only as the
person approves a proposal.

The agent is woken with a summary: each file's newest entries, at most twenty
each, and how many more there are. It reads the files themselves when it
needs the rest.

## Re-checked when their files change

`checked` keeps the default branch's commit the memory was last re-checked
against. Each time the agent wakes, and only then, devpit flags the entries
of the three files that rest on a file the default branch changed since that
commit. When any is flagged, the wake's first turn asks the agent to re-check
each against its files as they are now and answer `keep`, `remove` or
`update: <new text>`; the question follows in the next turn. Each stale entry
becomes a proposal in `proposed.md` that names the entry it replaces, and its
file is left as it is until the person decides. The commit is then noted as
checked. No timer re-checks: an agent that idles out waits for its next
question. The first wake of a memory kept from before only notes the commit.

## Lessons and decisions from closed cards

When a card lands in the done column, devpit:

1. Logs one line to `metrics.jsonl`: the day, the card, its review rounds
   (the runs its `[devflow] review:` comments name, each once) and the
   findings those rounds found again (`repeats` in each round's
   `findings.json` still kept), so the lessons' effect can be measured.
2. Asks the project agent, under the profile of an orchestrator linked to the
   project, what the card teaches. It reads the card's comments (its handoff,
   its review and gate results, the person's corrections) and its last
   review's findings, as data, and answers zero to three lessons; a choice
   the card settled that later work should follow comes as a decision.
3. Keeps them in `proposed.md`, each dated and with the card as its source.

Each proposal, a re-check's included, waits under the top bar and on its
card, with Approve, Edit and Discard. Approved, as it is or edited, a lesson
is appended to `lessons.md` and a decision to `decisions.md`, with the card
and the day; a re-check's takes the stale entry's place, or removes it.
Discarded, it is removed and nothing keeps it.

A decision names the cards that made it in its source, and a card shows the
decisions that name it, each with its cards.

The newest twenty approved lessons go into every brief the agent writes and
into every Devflow review of the project, as items of the checklist the change
is held to.

## Projects without an agent

A project gets `.devpit/project-agent/` only when its agent is first asked.
A closed card is measured and its lessons asked for only in a project that has
that folder, so a project whose agent was never asked works exactly as it did,
and every other tool keeps its shape.
