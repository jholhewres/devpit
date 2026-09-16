import type { HeadsDown, Notice } from '../gen/bindings'

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

/* `HeadsDown` itself comes from the contract — which project, and the second
   it began — because a type written twice is a type that disagrees with
   itself. */
export type { HeadsDown }

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
  if (!focus || focus.since === null) return false
  if (notice.createdAt === null) return false
  if (notice.createdAt < focus.since) return false
  if (notice.projectId === null) return false
  if (notice.projectId === focus.projectId) return false
  return notice.kind !== IRREVERSIBLE
}

/** How long the focus has been on, in whole minutes.
 *
 * `since` crosses the contract as `number | null`, because a float has values
 * JSON cannot carry. A focus with no beginning is one nothing can be counted
 * from, so it counts as none rather than as zero minutes of something. */
export const minutesIn = (focus: HeadsDown, now: number): number =>
  focus.since === null ? 0 : Math.max(0, Math.floor((now - focus.since) / 60))

/**
 * Which of this project's notices is the one that needs you next.
 *
 * The order is what somebody stuck between five agents actually wants: an
 * agent that is waiting on an answer first, because it is stopped until you
 * come; then a step with no undo, which is the one thing that fires without
 * being asked twice; then a run that ended; then everything else.
 *
 * Within a rank, what nobody has looked at comes before what they have, and
 * the older before the newer — the one that has been waiting longest.
 *
 * **A failed run is not ranked above one that finished**, and it should be.
 * The notice carries the sentence ("tests failed on …") and not the outcome,
 * so telling them apart here would mean reading English out of a title. That
 * is a change to what a notice records, not a regex — `notices.rs:104` writes
 * the sentence, and the rank can be right the day it writes the outcome too.
 */
const RANK: Readonly<Record<string, number>> = { agent: 0, irreversible: 1, run: 2 }

export function nextThatNeedsYou<T extends Pick<Notice, 'kind' | 'readAt' | 'createdAt'>>(
  mine: readonly T[],
): T | null {
  const ordered = [...mine].sort((a, b) => {
    const rank = (RANK[a.kind] ?? 9) - (RANK[b.kind] ?? 9)
    if (rank !== 0) return rank
    const unread = Number(a.readAt !== null) - Number(b.readAt !== null)
    if (unread !== 0) return unread
    return (a.createdAt ?? 0) - (b.createdAt ?? 0)
  })
  return ordered[0] ?? null
}

/** What the focus's own project did while the door was shut. */
export interface WhatYouDid {
  readonly ok: number
  readonly failed: number
  /** Only what actually cost something: a run with no figure is not a zero. */
  readonly costUsd: number | null
  /** How many runs carried no cost at all, so the sum can say it is partial. */
  readonly uncosted: number
}

/**
 * The inside half of the summary: what you did, not what was held.
 *
 * A run with no cost recorded is not counted as zero — a command step never
 * has one, and folding it in as zero would make the total read as complete
 * when it is a sum over agent turns only. `uncosted` is what lets the screen
 * say so instead of quietly rounding it away.
 */
export function whatYouDid(
  runs: readonly { run: { state: string; costUsd: number | null } }[],
): WhatYouDid {
  let ok = 0
  let failed = 0
  let costUsd: number | null = null
  let uncosted = 0

  for (const { run } of runs) {
    if (run.state === 'ok') ok += 1
    if (run.state === 'failed') failed += 1
    if (run.costUsd === null) uncosted += 1
    else costUsd = (costUsd ?? 0) + run.costUsd
  }

  return { ok, failed, costUsd, uncosted }
}
