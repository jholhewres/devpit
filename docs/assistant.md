# Schedules and Assistant mode

An orchestrator keeps the person's reminders and routines in one list, and
can be switched into Assistant mode, where it keeps an eye on time for them.
Only the person schedules: the orchestrator may propose a schedule, never
create one.

## Schedules

**Schedules** is an item in an orchestrator's sidebar. It lists:

- every **reminder** the orchestrator reaches — a card with a date, in the
  orchestrator or a project linked to it — still to come, gone off, or dealt
  with in the last seven days;
- every **routine** the person set: a morning summary, CI failures, pull
  requests waiting, or cards waiting in review, at a time on some days.

Each row shows when it goes off (day, time and zone), what it is about, its
project and card, whether it repeats, its state (waiting, went off, done or
paused) and when it last ran. From a row:

| | Reminder | Routine |
|---|---|---|
| Pause / Resume | — | stops it running; resumed, it runs at its next time, not at the ones it missed |
| Edit | moves its time | changes its kind, time and days |
| Resolve | marks it dealt with; the card keeps its date | — |
| Delete | takes the date off the card | removes it |

**New reminder** makes a card with a date in the orchestrator or a linked
project; **New routine** sets one up.

Routines and proposals are kept in the orchestrator's settings in devpit's
store, out of reach of its own chat. A routine's time is kept with the zone
and UTC offset the window gave when it was set; when the window sees that
zone's offset change, the routines in it follow.

## Assistant mode

A toggle in the orchestrator chat's corner, off by default and kept per
orchestrator. With it off, the orchestrator and its MCP tools are as they
were.

With it on:

- devpit writes `.devpit/assistant.md` in the orchestrator's folder — its
  own file, rewritten when devpit updates, like `.devpit/orchestrator.md` —
  and imports it into the orchestrator's `CLAUDE.md` on the line under
  `@.devpit/orchestrator.md`. Off, the import and the file go. The brief is
  read when the chat next starts; the person's own `CLAUDE.md` wins where the
  two differ.
- devpit's MCP server lists the assistant's tools. The toggle makes every
  server ask again what it offers, and the server sends
  `notifications/tools/list_changed`, so a running chat gains or loses them
  without a restart.

| Tool | Does |
|---|---|
| `devpit_schedules` | the Schedules list, and the proposals waiting |
| `devpit_schedule_propose` | proposes a routine or a reminder; it waits in Schedules until the person adds or drops it |

Called with the mode off, they are refused.
