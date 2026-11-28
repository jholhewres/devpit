# ADR 0003 — Agents mode: an orchestrator's agents do the work it hands out

- Status: **accepted** 2026-11-26
- Date: 2026-11-26

## Context

The person talks to an orchestrator, and the orchestrator starts sessions on
cards. They want more: an orchestrator that acts as their right hand, plans
the work and hands it to agents that persist, each with its own persona,
provider, memory and reach, while the orchestrator follows it up and reports.

devpit already holds most of what such an agent needs. An orchestrator is a
folder with a resident chat, a profile on any driver devpit runs (Claude Code,
Codex, the ACP agents), devpit's MCP server, briefs and links to projects.
What is new is an agent belonging to an orchestrator, and a mode that turns
that on.

Two recorded rules bound it (`docs/architecture.md`): nothing runs that the
person did not set up, and starting a step is the person's move. An agent that
could approve, start a step or reach production by itself would undo both.

## Decision

- **A mode per orchestrator, off by default.** Agents mode is the setting
  `agents` in the orchestrator's settings, kept in devpit's store out of its
  chat's reach, and switched only by the person's click.
- **Off is what it always was.** With the mode off, the orchestrator's
  `CLAUDE.md`, `AGENTS.md` and `.devpit/orchestrator.md` are byte for byte the
  files devpit wrote before the mode existed, and its MCP tool list and every
  response keep their shape. Tests pin both.
- **On is additive.** devpit writes `.devpit/agents.md`, a brief of its own,
  and imports it in `CLAUDE.md` under devpit's other briefs. The MCP server
  lists the agents' tool group, announced with `listChanged`. A new field or
  tool appears only with the mode on; an app too old to say the mode is never
  offered the group.
- **Agents belong to one orchestrator.** Each agent is a child of the
  orchestrator that made it: its own folder and `persona.md`, which only the
  person edits; a character avatar; a profile (provider, model, effort) on any
  driver devpit runs; a resident chat; a reach of projects, never wider than
  the orchestrator's; and memory. An agent is not shared between
  orchestrators.
- **The orchestrator plans and delegates.** It talks with the person, plans,
  hands the work out and follows it up, reporting once when what it handed out
  is done. It does the work itself only when it is trivial or only reads.
- **A delegated task is a card.** It goes on the project's board, or on the
  orchestrator's own when no project is concerned, with the agent as its
  owner. The board, the card's chat and Devflow are the task's, as for any
  card.
- **The brakes are the person's.**
  - No agent approves anything or answers a question that waits on the
    person.
  - No agent moves a card into a lane that runs a step.
  - No agent starts anything on a production host without the person's
    `confirm_production`.
  - A message between agents is marked as not the person's and never stands
    for their yes.
  - Nothing runs unless the person asked for it or scheduled it.
- **The store only grows.** The agents' records come in additive migrations;
  a store written before them opens as it was.

## Consequences

- The mode, its brief, its tool group and every later part of it are reached
  only through the setting. Turning it off takes the brief and the import away
  and leaves the files and the list as they were.
- An agent's session is a session like any other: on a card, on the board,
  under the same gates. The orchestrator stays the one that controls the
  projects' sessions.
- What only some drivers offer (transcripts, peers) shows as unavailable on
  the others, with nothing changing for the drivers that have it.
- Remote clients and old conversations keep working: what the mode adds is
  new, and nothing they read changes shape.
