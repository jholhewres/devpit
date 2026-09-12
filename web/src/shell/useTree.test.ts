import { describe, expect, it } from 'vitest'

import { isStale } from './useTree'

describe('whether a fetch response is still the current answer', () => {
  it('is stale once a newer reload has started', () => {
    expect(isStale(1, 2)).toBe(true)
  })

  it('is current when nothing has superseded it', () => {
    expect(isStale(2, 2)).toBe(false)
  })

  /* Proves the guard actually discriminates rather than always agreeing —
     the same generation on both sides must read as current. */
  it('is current for the very first fetch too', () => {
    expect(isStale(0, 0)).toBe(false)
  })
})
