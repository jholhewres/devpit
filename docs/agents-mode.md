# Agents mode

An orchestrator can be switched into Agents mode, where it becomes the
person's right hand: it talks with them, plans the work and hands it to agents
of its own, then follows it up and reports back. Decision record:
[ADR 0003](adr/0003-agents-belong-to-an-orchestrator.md).

## The toggle

A toggle in the orchestrator chat's corner, beside Assistant mode, off by
default and kept per orchestrator in devpit's store, out of reach of the
orchestrator's own chat. Only the person's click changes it.

### Off

Nothing changes. The orchestrator's `CLAUDE.md`, `AGENTS.md` and
`.devpit/orchestrator.md` are the files devpit always wrote, byte for byte,
and its MCP tool list is the one it always had. An app too old to know the
mode never offers its tools.

### On

- devpit writes `.devpit/agents.md` in the orchestrator's folder — its own
  file, rewritten when devpit updates, like `.devpit/orchestrator.md` — and
  imports it into the orchestrator's `CLAUDE.md` under devpit's other briefs:
  after `@.devpit/orchestrator.md`, and after `@.devpit/assistant.md` when
  Assistant mode is on too. `AGENTS.md`, which Codex and the ACP agents read,
  carries it as well. Off, the import and the file go. The brief is read when
  the chat next starts; the person's own `CLAUDE.md` wins where the two
  differ.
- devpit's MCP server lists the agents' tools. The toggle makes every server
  ask again what it offers, and the server sends
  `notifications/tools/list_changed`, so a running chat gains or loses them
  without a restart.

| Tool | Does |
|---|---|
| `devpit_agents` | the orchestrator's agents, each with its handle, name and role |

Called with the mode off, they are refused.

## What the brief tells the orchestrator

- It plans and delegates, and does the work itself only when it is trivial or
  only reads.
- A task it hands out is a card: on the project's board when it is about a
  project, on the orchestrator's own board when it is not, with the agent that
  does it as its owner. With no agent for it yet, it delegates as it always
  did: a card, and a session started on it.
- It reports once, when everything it handed out is done.

## What never changes

- Nothing runs that the person did not ask for or schedule.
- No agent approves anything for the person or answers a question that waits
  on them.
- No agent moves a card into a lane that runs a step.
- No agent starts anything on a production host without the person's
  `confirm_production`.
- A message from an agent is not the person, and never stands for their yes.
- Only the person creates or changes an agent, or widens what it reaches.
