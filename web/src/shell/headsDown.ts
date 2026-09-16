import type { Notice } from '../gen/bindings'

/*
 * Focus, and the one rule it is made of.
 *
 * `focus` already means something here — focusing a tab or a pane
 * (`shape.ts:51`) — so this is `headsDown` in the code and "Focus" on screen.
 *
 * What it is: a door with a queue behind it, for one project. Nothing about
 * that project leaves the screen; what changes is that something from
 * somewhere else stops calling for attention until you come out. The work
 * itself never stops — lanes set to advance go on advancing, runs finish,
 * agents on other projects keep going. Only the calling waits.
 */

export type HeadsDown = {
  /** The project being worked on. Focus is per project, never per card: the
   *  work is several cards of one project at once. */
  projectId: string
  /** Seconds since the epoch, so the clock survives a restart. */
  since: number
}

/** A notice's kind that goes through the door whatever else is true. */
const IRREVERSIBLE = 'irreversible'

/**
 * Whether this notice waits for the end of the focus instead of ringing now.
 *
 * Held is not read, and never dropped: it is shown on the way out, grouped by
 * project, and stays unread until somebody acts on it.
 *
 * Three things are never held:
 *
 * - anything from before the focus began, which was already on screen;
 * - this project's own, which is the whole point of being in it;
 * - a step with no undo, from anywhere, because that is where waiting costs
 *   the most.
 *
 * And a notice with no project at all is not held either. Its project is
 * unknown rather than different — it comes from a card whose project could not
 * be resolved (`notices.rs:104`, `reconcile.rs:38`) — and a summary grouped by
 * project would file it under nothing, where nobody would look for it. One
 * with no timestamp is the same case: without it there is no telling whether
 * it arrived during the focus, and what cannot be placed is not hidden.
 */
export function held(
  notice: Pick<Notice, 'projectId' | 'kind' | 'createdAt'>,
  focus: HeadsDown | null,
): boolean {
  if (!focus) return false
  if (notice.createdAt === null) return false
  if (notice.createdAt < focus.since) return false
  if (notice.projectId === null) return false
  if (notice.projectId === focus.projectId) return false
  return notice.kind !== IRREVERSIBLE
}

/** How long the focus has been on, in whole minutes. */
export const minutesIn = (focus: HeadsDown, now: number): number =>
  Math.max(0, Math.floor((now - focus.since) / 60))
