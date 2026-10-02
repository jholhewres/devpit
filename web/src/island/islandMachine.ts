/*
 * When the island shows, and how much of it.
 *
 * Three sizes. Hidden is nothing but a strip at the top of the screen that
 * the cursor can wake. Compact is a pill: who is working, who waits on you.
 * Expanded is the whole island. Work going on reveals the compact; something
 * waiting on a person opens it all the way, because that is the one thing
 * worth interrupting somebody for.
 *
 * Pure, so the timing is tested rather than watched: each nudge gives the
 * next island, and `due` says when nothing happening would change it.
 */

export type Mode = 'hidden' | 'compact' | 'expanded'

export interface Island {
  readonly mode: Mode
  /** When the mode last changed, or the cursor last left: what timers count from. */
  readonly since: number
  readonly hovering: boolean
  /** Whether the cursor has been on it since it opened: leaving then means done with it. */
  readonly touched: boolean
  readonly pinned: boolean
}

export type Nudge =
  | { readonly kind: 'activity' }
  | { readonly kind: 'alert' }
  | { readonly kind: 'enter' }
  | { readonly kind: 'leave' }
  | { readonly kind: 'open' }
  | { readonly kind: 'close' }
  | { readonly kind: 'pin'; readonly pinned: boolean }
  | { readonly kind: 'tick' }
  /** devpit was paused: down, whatever it was doing. */
  | { readonly kind: 'rest' }

/** How long the open island waits for somebody when it opened on its own. */
export const EXPANDED_FOR = 15_000
/** How long it stays open once the cursor that was on it has gone. */
export const LEFT_FOR = 1_400
/** How long the pill stays with nothing new happening. */
export const COMPACT_FOR = 60_000

export const resting = (now: number): Island => ({ mode: 'hidden', since: now, hovering: false, touched: false, pinned: false })

/**
 * The island after one nudge. `held` is whether something waits on the
 * person: while it does, the pill does not go away on its own.
 */
export function next(island: Island, nudge: Nudge, now: number, held: boolean): Island {
  const to = (mode: Mode): Island => ({ ...island, mode, since: now, touched: island.hovering })
  switch (nudge.kind) {
    case 'activity':
      return island.mode === 'hidden' ? to('compact') : island.mode === 'compact' ? { ...island, since: now } : island
    case 'alert':
      return to('expanded')
    case 'enter':
      return { ...(island.mode === 'hidden' ? to('compact') : island), hovering: true, touched: true }
    case 'leave':
      return { ...island, hovering: false, since: now }
    case 'open':
      return to('expanded')
    case 'close':
      return { ...to('compact'), pinned: false }
    case 'rest':
      return resting(now)
    case 'pin':
      return { ...island, pinned: nudge.pinned, since: now }
    case 'tick': {
      const at = due(island, held)
      if (at === null || now < at) return island
      return island.mode === 'expanded' ? to('compact') : to('hidden')
    }
  }
}

/** When the island would change on its own, or `null` while it will not. */
export function due(island: Island, held: boolean): number | null {
  if (island.hovering || island.pinned) return null
  if (island.mode === 'expanded') return island.since + (island.touched ? LEFT_FOR : EXPANDED_FOR)
  if (island.mode === 'compact' && !held) return island.since + COMPACT_FOR
  return null
}
