import type { ConversationMarks } from '../gen/bindings'

/*
 * Where a conversation sits in the sidebar: in progress, or settled — by hand,
 * or snoozed until a time still ahead. Times are Unix seconds, as kept.
 */

export const isSettled = (marks: ConversationMarks | undefined, now: number): boolean =>
  Boolean(marks && (marks.settled || (marks.snoozedUntil ?? 0) > now))

/* Pinned first within each group; the rest keep the order they came in. */
export function grouped<T>(
  rows: readonly T[],
  marksOf: (row: T) => ConversationMarks | undefined,
  now: number,
): { readonly progress: T[]; readonly settled: T[] } {
  const pinnedFirst = [...rows].sort((a, b) => Number(Boolean(marksOf(b)?.pinned)) - Number(Boolean(marksOf(a)?.pinned)))
  return {
    progress: pinnedFirst.filter((row) => !isSettled(marksOf(row), now)),
    settled: pinnedFirst.filter((row) => isSettled(marksOf(row), now)),
  }
}

export type Snooze = 'hour' | 'tomorrow'

/* An hour from now, or nine tomorrow morning on the reader's own clock. */
export function snoozeUntil(snooze: Snooze, now: Date): number {
  if (snooze === 'hour') return Math.floor(now.getTime() / 1000) + 3600
  return Math.floor(new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1, 9).getTime() / 1000)
}
