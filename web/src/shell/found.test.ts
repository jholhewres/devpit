import { describe, expect, it } from 'vitest'

import type { Found } from '../gen/bindings'
import { leftNoReview, placeOf, severityWord, standingWords, worstFirst } from './found'

/*
 * The sentences are the feature. The one that matters: a review made against
 * another revision is old, not wrong, and its lines mean the commit it was
 * made against.
 */

const found = (over: Partial<Found> = {}): Found => ({
  findings: [],
  standing: 'current',
  atRevision: '2f0bee9abc',
  now: '2f0bee9abc',
  rubric: null,
  ...over,
})

const finding = (severity: Found['findings'][number]['severity'], file: string, line: number | null) => ({
  file,
  line,
  severity,
  why: 'because',
})

describe('what a review found, in words', () => {
  it('gives every severity a word, so none is colour alone', () => {
    for (const severity of ['blocking', 'worth', 'noted'] as const) {
      expect(severityWord(severity).length, severity).toBeGreaterThan(0)
    }
  })

  it('sorts the way somebody acts on them: worst first', () => {
    const sorted = worstFirst(
      found({
        findings: [
          finding('noted', 'c.rs', 1),
          finding('blocking', 'b.rs', 2),
          finding('worth', 'a.rs', 3),
        ],
      }),
    )
    expect(sorted.map((one) => one.severity)).toEqual(['blocking', 'worth', 'noted'])
  })

  it('writes a place the way an editor does, and a whole-file finding without one', () => {
    expect(placeOf('src/main.rs', 12)).toBe('src/main.rs:12')
    expect(placeOf('src/main.rs', null)).toBe('src/main.rs')
  })

  /* Sabotage: say "these findings may be wrong" and a person deletes work that
     is still a problem. Old is not wrong. */
  it('calls an outdated review old and never wrong', () => {
    const said = standingWords('outdated', found({ standing: 'outdated', now: 'ed16baf000' }))!
    expect(said).not.toMatch(/wrong|invalid|no longer true/i)
    expect(said).toMatch(/2f0bee9abc/)
    expect(said).toMatch(/ed16baf000/)
  })

  /* Following a line through a diff is a guess dressed as a fact, and the
     screen says out loud that it does not. */
  it('says out loud that it does not follow a line', () => {
    expect(standingWords('outdated', found({ standing: 'outdated' }))!).toMatch(
      /does not follow it/i,
    )
  })

  /* A caveat on every review teaches people to ignore the ones that matter. */
  it('says nothing about a review of the code that is checked out', () => {
    expect(standingWords('current', found())).toBe(null)
  })

  it('says nothing about an unanchored review that found nothing', () => {
    expect(standingWords('unanchored', found({ standing: 'unanchored' }))).toBe(null)
    expect(
      standingWords(
        'unanchored',
        found({ standing: 'unanchored', findings: [finding('noted', 'a.rs', 1)] }),
      ),
    ).toMatch(/cannot be checked/i)
  })

  /* A run that left no review is not a review that found nothing: one looked
     at something and the other did not. */
  it('tells no review from a review that found nothing', () => {
    expect(leftNoReview({ findings: [], standing: 'unanchored', atRevision: null, now: null, rubric: null })).toBe(true)
    expect(leftNoReview(found({ findings: [] }))).toBe(false)
  })
})
