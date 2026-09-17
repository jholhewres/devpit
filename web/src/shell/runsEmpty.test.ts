import { describe, expect, it } from 'vitest'

import { emptyBecause, narrowed, type RunFilters } from './runs'

/*
 * The three empties are three different sentences, and telling them apart is
 * the whole point: "No runs match" on a project that cannot run anything sends
 * somebody hunting a filter that is not the problem.
 */

const nothing: RunFilters = { stepId: null, state: null, from: '', to: '' }

describe('why the runs list is empty', () => {
  it('is not empty when there are rows', () => {
    expect(emptyBecause(2, nothing, false)).toBe(null)
    expect(emptyBecause(0, nothing, false)).toBe(null)
  })

  /* Sabotage: answer 'noneMatch' here and the offer to configure a step never
     appears on the project that needs it. */
  it('says a board with no lane that runs anything has nothing configured', () => {
    expect(emptyBecause(0, nothing, true)).toBe('noChecks')
    expect(emptyBecause(0, { ...nothing, state: 'failed' }, true)).toBe('noChecks')
  })

  it('says a board that has a check and never ran it is waiting, not broken', () => {
    expect(emptyBecause(1, nothing, true)).toBe('nothingRan')
  })

  it('blames the filter only when there is a filter to blame', () => {
    expect(emptyBecause(1, { ...nothing, state: 'failed' }, true)).toBe('noneMatch')
    expect(emptyBecause(1, { ...nothing, stepId: 'step_1' }, true)).toBe('noneMatch')
    expect(emptyBecause(1, { ...nothing, from: '2026-09-01' }, true)).toBe('noneMatch')
    expect(emptyBecause(1, { ...nothing, to: '2026-09-01' }, true)).toBe('noneMatch')
  })

  it('knows when nothing is narrowing the list', () => {
    expect(narrowed(nothing)).toBe(false)
    expect(narrowed({ ...nothing, state: 'ok' })).toBe(true)
  })
})
