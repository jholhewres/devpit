import type { PaneName } from './paneList'

/*
 * What opening and closing a pane does to the strip.
 *
 * These are functions rather than lines inside the hook so a test can call
 * them. A test that re-implements "closing the active one lands on the
 * neighbour" passes whether or not the app still does that.
 */

export interface Strip {
  readonly open: readonly PaneName[]
  readonly active: PaneName | null
}

/** Appended, never moved: a tab keeps the place it was given. */
export function opened(strip: Strip, name: PaneName): Strip {
  return {
    open: strip.open.includes(name) ? strip.open : [...strip.open, name],
    active: name,
  }
}

/** Closing the one you are looking at lands on the neighbour, not on nothing. */
export function closed(strip: Strip, name: PaneName): Strip {
  const at = strip.open.indexOf(name)
  if (at < 0) return strip
  const open = strip.open.filter((other) => other !== name)
  return {
    open,
    active: strip.active === name ? (open[Math.min(at, open.length - 1)] ?? null) : strip.active,
  }
}

/** A drag puts the tab where it was dropped and leaves the rest in order. */
export function moved(strip: Strip, name: PaneName, to: number): Strip {
  const at = strip.open.indexOf(name)
  if (at < 0 || at === to || to < 0 || to >= strip.open.length) return strip
  const open = [...strip.open]
  open.splice(at, 1)
  open.splice(to, 0, name)
  return { ...strip, open }
}
