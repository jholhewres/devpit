import type { PaneName } from './paneList'

/*
 * A tab is an instance, not a kind.
 *
 * The first cut keyed tabs by pane name, so there could only ever be one
 * terminal and "new terminal" focused the one you had. Terminals and chats
 * are things you open several of; the board and the file tree are one each.
 */
export interface Tab {
  readonly id: string
  readonly kind: PaneName
  /** A terminal says where it is; a board is just the board. */
  readonly title?: string
  /** The backend leaves a terminal tab is showing, in drawing order.

      A list because a tab is a tree: splitting gives it a second pane, and a
      tab that recorded one would stop knowing what half of itself is doing
      the moment it was split. */
  readonly panes?: readonly string[]
  /** The file a `file` tab is showing, relative to the project root. */
  readonly path?: string
  /** An agent this terminal was opened in order to run.

      Carried on the tab because the pane it will run in does not exist yet:
      picking `Claude Code` from the menu opens a terminal, and the terminal
      asks the backend for a tree before there is anything to type into.
      Cleared the moment it is sent, so it cannot be sent twice. */
  readonly launch?: string
}

export interface Strip {
  readonly open: readonly Tab[]
  readonly active: string | null
}

/** Kinds you can have several of. Everything else focuses what is open. */
const MANY: ReadonlySet<PaneName> = new Set<PaneName>(['term', 'chat', 'file', 'diff'])

export const many = (kind: PaneName): boolean => MANY.has(kind)

export function opened(strip: Strip, tab: Tab): Strip {
  /* An id that is already open focuses it. A file tab's id is its path, so
     opening the same file twice lands on the tab you already have rather than
     giving you two views of one file that can disagree. */
  const same = strip.open.find((other) => other.id === tab.id)
  if (same) return { ...strip, active: same.id }

  if (!many(tab.kind)) {
    const already = strip.open.find((other) => other.kind === tab.kind)
    /* Appended, never moved: a tab keeps the place it was given. */
    if (already) return { ...strip, active: already.id }
  }
  /* A terminal says which one it is from the moment it opens. A chat waits:
     it is named after the first thing you say in it, and until then "Chat" is
     the truest label available. */
  const named =
    tab.title === undefined && tab.kind === 'term'
      ? { ...tab, title: nextName(strip.open, 'term', 'Terminal') }
      : tab
  return { open: [...strip.open, named], active: named.id }
}

export function closed(strip: Strip, id: string): Strip {
  const at = strip.open.findIndex((tab) => tab.id === id)
  if (at < 0) return strip
  const open = strip.open.filter((tab) => tab.id !== id)
  return {
    open,
    /* Closing the one you are looking at lands on the neighbour. */
    active: strip.active === id ? (open[Math.min(at, open.length - 1)]?.id ?? null) : strip.active,
  }
}

export function moved(strip: Strip, id: string, to: number): Strip {
  const at = strip.open.findIndex((tab) => tab.id === id)
  if (at < 0 || at === to || to < 0 || to >= strip.open.length) return strip
  const open = [...strip.open]
  const [tab] = open.splice(at, 1)
  open.splice(to, 0, tab!)
  return { ...strip, open }
}

export function renamed(strip: Strip, id: string, title: string): Strip {
  return { ...strip, open: strip.open.map((tab) => (tab.id === id ? { ...tab, title } : tab)) }
}

/* What a conversation is called: the first thing you said in it.

   Their own words rather than a generated summary — the point of the name is
   that you recognise it, and a title written by a model is one more thing
   that can be wrong about a conversation you remember perfectly. This is the
   same rule `agentcli::history::title_of` applies to the transcripts on disk,
   so a conversation is called the same thing in the tab and in the list of
   earlier ones. */
export function titleOf(said: string, limit = 48): string {
  const first = said.split('\n').map((line) => line.trim()).find((line) => line !== '') ?? ''
  const short = [...first].slice(0, limit).join('').trimEnd()
  if (short === '') return ''
  return [...first].length > limit ? `${short}…` : short
}

/* The next free number for a kind, so a second terminal is Terminal 2.

   Lowest unused rather than one past the highest: closing Terminal 1 and
   opening another should give you Terminal 1 back, not Terminal 4 beside a
   Terminal 2. */
export function nextName(open: readonly Tab[], kind: PaneName, base: string): string {
  const taken = new Set(
    open.flatMap((tab) => {
      if (tab.kind !== kind || !tab.title) return []
      const match = new RegExp(`^${base} (\\d+)$`).exec(tab.title)
      return match ? [Number(match[1])] : []
    }),
  )
  let at = 1
  while (taken.has(at)) at += 1
  return `${base} ${at}`
}

/* Records which backend leaves a terminal tab is showing.

   Unchanged when the list is the same, because this is called from a render
   that reads the tree, and a new array every time would be a new strip every
   time — which is a write to storage and a repaint of every tab. */
export function attached(strip: Strip, id: string, panes: readonly string[]): Strip {
  const tab = strip.open.find((one) => one.id === id)
  if (!tab || same(tab.panes, panes)) return strip
  return { ...strip, open: strip.open.map((one) => (one.id === id ? { ...one, panes } : one)) }
}

const same = (a: readonly string[] | undefined, b: readonly string[]): boolean =>
  a !== undefined && a.length === b.length && a.every((one, at) => one === b[at])

/* The agent has been sent to the pane, so the tab stops asking for it. */
export function launched(strip: Strip, id: string): Strip {
  return {
    ...strip,
    open: strip.open.map((tab) => {
      if (tab.id !== id || tab.launch === undefined) return tab
      const { launch: _sent, ...rest } = tab
      return rest
    }),
  }
}

export const focused = (strip: Strip): Tab | null =>
  strip.open.find((tab) => tab.id === strip.active) ?? null

/* Two clicks close together on the same thing.

   Read from the clicks rather than taken from `dblclick`: a tab captures the
   pointer to be dragged, and a captured pointer retargets the events the
   double-click is derived from — so the browser's own event never arrives.
   The threshold is the platform's usual one. */
export const DOUBLE_MS = 400

export interface Clicked {
  readonly id: string
  readonly at: number
}

export const twice = (last: Clicked | null, id: string, at: number): boolean =>
  last !== null && last.id === id && at - last.at <= DOUBLE_MS

/* What a name is cut to before it is drawn.

   A tab is a fixed width and a sidebar row is a column: a name that runs past
   either pushes the things beside it off the screen. Cut in characters rather
   than left to CSS ellipsis alone, so the tab keeps its size in the strip. */
export function short(name: string, limit: number): string {
  const letters = [...name]
  return letters.length > limit ? `${letters.slice(0, limit).join('').trimEnd()}…` : name
}

/* The tab a backend pane is drawn in, or nothing when no open tab shows it.

   A pane is not a tab: a split tab shows several, and the resource monitor
   names panes because that is what it measures. Handing a pane id to `focus`,
   which takes a tab id, focused nothing at all. */
export function tabOfPane(open: readonly Tab[], paneId: string): string | null {
  return open.find((tab) => tab.panes?.includes(paneId))?.id ?? null
}

