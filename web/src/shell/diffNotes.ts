import type { CardFindings, Finding, Severity, Standing } from '../gen/bindings'
import type { Hunk } from './diff'

/*
 * A review's findings, placed on the lines of a card's diff.
 *
 * Placed by the line the finding names on the new side, and never moved: an
 * outdated finding keeps its number and says which revision it meant.
 */

export interface Note {
  readonly runId: string
  /** Its place in its run's review, which is how it is set aside. */
  readonly at: number
  readonly finding: Finding
  readonly standing: Standing
  readonly atRevision: string | null
  readonly dismissed: boolean
}

export const notesOf = (found: CardFindings): Note[] =>
  found.reviews.flatMap((review) =>
    review.found.findings.map((finding, at) => ({
      runId: review.runId,
      at,
      finding,
      standing: review.found.standing,
      atRevision: review.found.atRevision,
      dismissed: review.dismissed.includes(at),
    })),
  )

/* The new side's number for each row of a hunk; `null` for a removed line. */
export function newLines(hunk: Hunk): (number | null)[] {
  let line = Number(/\+(\d+)/.exec(hunk.header)?.[1] ?? 0)
  return hunk.rows.map((row) => (row.kind === 'del' ? null : line++))
}

/* What is still standing, by severity: what the counter over the diff says. */
export function tally(notes: readonly Note[]): Readonly<Record<Severity, number>> {
  const counts = { blocking: 0, worth: 0, noted: 0 }
  for (const note of notes) if (!note.dismissed) counts[note.finding.severity] += 1
  return counts
}

/* A shape as well as a colour, so the severity is never colour alone. */
export const SHAPE: Readonly<Record<Severity, string>> = { blocking: '■', worth: '▲', noted: '●' }
