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
and UTC offset the window gave when it was set. The orchestrator's chat gives
the window's zone again as it opens and each time the window comes back, so a
clock change moves the routines set in that zone with it.

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
| `devpit_heartbeat_status` | how the heartbeat is set, when it last looked and what it found |
| `devpit_briefing` | what happened in the last hours, in a few lines |
| `devpit_telegram_send` | a message to the person's Telegram, as much as the chat's privacy lets out; refused while devpit is paused |

Called with the mode off, they are refused.

## Routines running

devpit looks once a minute. A routine whose time came runs, unless it is
paused, and moves to its next time; its last run shows on its row. What it
says goes to the bell, to the person's channels under "Your assistant has
something for you" in Settings → Channels, and, when devpit is behind, to a
system notification:

| Routine | Says |
|---|---|
| Morning summary | the comeback briefing since its last run |
| CI failures | the open pull requests of the linked projects whose checks fail |
| Pull requests waiting | the open pull requests of the linked projects |
| Cards waiting in review | the cards in a lane whose role is check |

Pull requests are read with `gh` in each linked project's folder. Channel
rules saved before this event existed leave it off until the person ticks it.

## The heartbeat

While the mode is on, the heartbeat looks every few minutes (Settings →
Assistant: every 5 to 60 minutes) for what needs the person, each check on or
off there:

- CI failing on an open pull request of a linked project;
- a session stopped on a question for longer than a set time;
- a card in a lane whose role is doing that waits on a card still holding it.

It stays silent when it finds nothing, and when it finds only what it found at
its last look; a look that could not read a source (`gh` timing out, say)
keeps what the last one found there, so it is not said again. Something new is
said once. Each session's question is a notice of its own and goes with its
terminal, so on Telegram its first choices are buttons, and a press answers
it the way the Sessions panel does; the rest go together. While devpit is
paused it holds what it would say. With the mode off, nothing looks. Nothing
here starts work.

## The comeback briefing

Back in the window after half an hour away, an orchestrator in Assistant mode
shows above its composer what happened meanwhile, in three lines: the
sessions that ran, waited and ended; the pull requests merged in its linked
projects; and the decisions waiting — memory proposals, schedule proposals,
drafts to send and Devflow plans to approve.

## Telegram both ways

Notices go out on Telegram as the channel rules say, and the person answers
from there: a session's question by its buttons, a draft by its Send button,
and anything else by writing to the orchestrator, which gets it as their turn.
