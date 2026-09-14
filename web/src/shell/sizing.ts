/*
 * How wide the two side panels are.
 *
 * Both live in one CSS grid — `sidebar | content | files` — so a width is not
 * only its own business: the content between them has a minimum, and a drag
 * that ignores it squeezes the terminal to nothing rather than stopping. Every
 * answer here is therefore clamped against the window as well as against the
 * panel's own limits.
 */

/** What a panel is when nobody has said otherwise. */
export const WIDE = { sidebar: 252, files: 340 } as const

/** Narrow enough to be worth doing, wide enough to still be a panel. */
export const LEAST = { sidebar: 180, files: 240 } as const
export const MOST = { sidebar: 480, files: 640 } as const

/** What the middle keeps, whatever the panels want. Matches `minmax` in the
 *  grid, and the grid would win anyway — this makes the drag stop instead. */
export const CONTENT_LEAST = 360

export type Panel = keyof typeof WIDE

export interface Widths {
  readonly sidebar: number
  readonly files: number
}

const between = (low: number, value: number, high: number): number =>
  Math.max(low, Math.min(high, Math.round(value)))

/**
 * A width, held to what this window can actually give it.
 *
 * `other` is what the panel on the far side is taking, because the room left
 * for this one is what the window has minus that panel and minus the content's
 * minimum. Without it, dragging the sidebar wide on a narrow window pushes the
 * files panel off the screen instead of stopping.
 */
export function held(panel: Panel, wanted: number, window: number, other: number): number {
  const room = window - other - CONTENT_LEAST
  return between(LEAST[panel], Math.min(wanted, room), MOST[panel])
}

/** Both widths, held — for a window that has just been made smaller. */
export function fits(widths: Widths, window: number): Widths {
  const sidebar = held('sidebar', widths.sidebar, window, LEAST.files)
  return { sidebar, files: held('files', widths.files, window, sidebar) }
}

/**
 * Where a drag puts the edge.
 *
 * The sidebar grows as the pointer moves right and the files panel grows as it
 * moves left, which is the one asymmetry: they are opposite edges of the same
 * window.
 */
export function dragged(
  panel: Panel,
  startedAt: number,
  wasWide: number,
  now: number,
): number {
  const moved = now - startedAt
  return wasWide + (panel === 'sidebar' ? moved : -moved)
}

/** What a width read back from disk means, including nothing at all. */
export function stored(sidebar: number | null, files: number | null): Widths {
  return {
    sidebar: sidebar === null ? WIDE.sidebar : between(LEAST.sidebar, sidebar, MOST.sidebar),
    files: files === null ? WIDE.files : between(LEAST.files, files, MOST.files),
  }
}
