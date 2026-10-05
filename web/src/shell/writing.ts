/*
 * The pace an answer is written at on screen.
 *
 * A model sends text in bursts — a sentence, then nothing, then a paragraph —
 * and drawn as it lands that reads as text being pasted, not written. So what
 * is shown trails what arrived, closing the gap in about LAG_MS, never slower than
 * FLOOR_CPS, and always up to the end of a word.
 */

const FLOOR_CPS = 120
const LAG_MS = 300
/* How far past the pace a word may run before it is cut instead. */
const LONGEST_WORD = 16

export interface Pace {
  readonly live: boolean
  readonly reduced: boolean
}

/* How much of `text` to show after `elapsedMs`, from `shown`. */
export function typed(text: string, shown: number, elapsedMs: number, pace: Pace): number {
  const arrived = text.length
  // Finished, or asked not to animate: whatever has arrived is shown.
  if (!pace.live || pace.reduced || shown >= arrived) return arrived
  const backlog = arrived - shown
  const cps = Math.max(FLOOR_CPS, (backlog * 1000) / LAG_MS)
  const next = Math.min(arrived, shown + Math.max(1, Math.round((cps * elapsedMs) / 1000)))
  return wordEnd(text, next)
}

/* `at`, moved forward to the end of the word it falls in. */
export function wordEnd(text: string, at: number): number {
  let end = at
  while (end < text.length && end - at < LONGEST_WORD && !/\s/.test(text[end]!)) end += 1
  return end - at < LONGEST_WORD ? end : at
}
