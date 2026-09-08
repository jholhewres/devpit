# The model

Seven objects. If something is not one of these, it does not get in.

```
project ──┬── target terminal   (one, and only one)
          ├── board ── column ──┬── card ── run*
          │                     └── step?
          └── session*
```

## `project`

A git repository and its preferences. **The axis of parallelism is the
project**, not the branch — the hard part is not producing five versions of one
feature, it is not losing the thread when you switch projects and come back two
days later.

## `terminal` — the target

One tmux *window* per project. The layout is drawn by the application, not by
tmux: a tmux split would show every leaf in a single client and fight the layout
the application persists.

Output is read through `pipe-pane`, which needs no attached client — recognition
has to work with the window closed, or a notification can never fire.

## `board`, `column`

A project's columns, in the order they appear.

**Columns are data, not code.** You create, rename, reorder and delete them, and
you decide whether a column runs a step and which kind. The default board of a
new project is a starting point, and nothing in the code may assume its column
names.

## `card`

The unit of work: title, markdown description, the column it sits in.

**The card is the trigger, not the record.** That distinction is what keeps it
from going stale: a board that merely describes what happens elsewhere is out of
date by the first busy day. If moving the card is what starts the work, it has
no way to lie.

## `step`

A column's rule: *on arrival here, run this*. A step is one of three kinds.

| | `agent` | `session` | `command` |
|---|---|---|---|
| Takes the target terminal | no | yes | no |
| Good for | refine, review, verify | implementing | tests, builds, deploys |
| Returns | schema-validated JSON | a session you drive | streamed output and an exit code |
| Concurrent | several | one | several |

A `command` step receives its context **only through environment variables**,
never interpolated into the command string. A branch name containing a space or
a `;` becomes a variable's value rather than shell syntax, which removes an
entire class of injection.

Output streams while the command runs. A test suite that takes twenty minutes
shows its first line immediately, not twenty minutes from now.

## `run`

One execution of a step, and what it cost: agent or command, input, output,
verdict, spend, duration, turn count, denied permissions, exit code.

A card shows its runs. That is how "the agent is doing something" becomes a
sentence with numbers in it.

## `session`

A background agent session, tied to a card and a worktree.

**The application does not own session state.** The agent CLI already answers
whether a session is busy or idle; keeping that in a database would create a
second truth that drifts from the first. The database stores only the link —
which card, which worktree.

The same holds for what a session cost: it is read back from the session's own
transcript, so a session you drove yourself, with nothing watching, still
reports its spend.

## What is not an object

Open file, tab, pane, window layout. That is interface state and it lives with
the interface. Promoting it to a domain object is how a workspace for agents
turns into an IDE — and an IDE is the weight this project is running from.
