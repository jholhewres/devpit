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
  /** The backend leaf a terminal is attached to. */
  readonly paneId?: string
  /** The file a `file` tab is showing, relative to the project root. */
  readonly path?: string
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
  return { open: [...strip.open, tab], active: tab.id }
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

/** Records which backend leaf a terminal tab took, so a reopen finds it. */
export function attached(strip: Strip, id: string, paneId: string): Strip {
  return { ...strip, open: strip.open.map((tab) => (tab.id === id ? { ...tab, paneId } : tab)) }
}

export const focused = (strip: Strip): Tab | null =>
  strip.open.find((tab) => tab.id === strip.active) ?? null
