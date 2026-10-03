import type { Tested } from '../gen/bindings'

/* A run's test report, in words. */

/** Whether the run left no report at all. */
export const leftNoReport = (tested: Tested): boolean => tested.readFrom.length === 0

/** The counts as a sentence, naming where they came from. */
export function countsWords(tested: Tested): string {
  const parts = [`${tested.passed} passed`, `${tested.failed} failed`]
  if (tested.skipped > 0) parts.push(`${tested.skipped} skipped`)
  return `${parts.join(', ')} — read from ${tested.readFrom.join(', ')}.`
}

/** Said when the list is shorter than the count. */
export function unlistedWords(tested: Tested): string | null {
  const unlisted = tested.failed - tested.failures.length
  return unlisted > 0 ? `And ${unlisted} more not listed.` : null
}
