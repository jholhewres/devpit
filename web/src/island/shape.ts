import type { Mode } from './islandMachine'

/*
 * How big the island is drawn, and where in its window.
 *
 * The window is as big as the largest view and never moves; the island is a
 * capsule inside it, floating a little below the top edge in the middle.
 * These numbers are the whole layout, so the window's own size in
 * `island.rs` has to fit them.
 */

export const WINDOW_WIDE = 760
/** The gap between the top of the screen's free area and the capsule. */
export const TOP = 8
/** One session's row in the overview. */
export const ROW = 48
/** The most rows the overview shows; the rest are counted. */
export const ROWS = 4

/** A rectangle in the window's logical pixels; it is what `island_shape` takes. */
export interface Rect {
  readonly x: number
  readonly y: number
  readonly width: number
  readonly height: number
}

export type View = 'overview' | 'session' | 'asking'

export interface Size {
  readonly width: number
  readonly height: number
  readonly radius: number
}

/** The strip at the top that wakes a hidden island, and nothing else takes the mouse. */
export const WAKE: Rect = { x: (WINDOW_WIDE - 240) / 2, y: 0, width: 240, height: 4 }

/** How big the island is, given what it shows; the overview grows with its rows. */
export function sizeOf(mode: Mode, view: View, rows: number): Size {
  if (mode === 'hidden') return { width: 0, height: 0, radius: 0 }
  if (mode === 'compact') return { width: 340, height: 40, radius: 20 }
  if (view === 'session') return { width: 660, height: 300, radius: 22 }
  if (view === 'asking') return { width: 560, height: 166, radius: 22 }
  return { width: 600, height: 70 + Math.max(1, Math.min(rows, ROWS)) * ROW, radius: 22 }
}

/** Where the island sits in its window, for the part that takes the mouse. */
export function rectOf(size: Size): Rect {
  return { x: (WINDOW_WIDE - size.width) / 2, y: TOP, width: size.width, height: size.height }
}

/** Both rects at once: what takes the mouse while one size turns into another. */
export function spanning(one: Rect, other: Rect): Rect {
  if (one.width === 0) return other
  if (other.width === 0) return one
  const x = Math.min(one.x, other.x)
  const y = Math.min(one.y, other.y)
  return {
    x,
    y,
    width: Math.max(one.x + one.width, other.x + other.width) - x,
    height: Math.max(one.y + one.height, other.y + other.height) - y,
  }
}
