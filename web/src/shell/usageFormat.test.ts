import { describe, expect, it } from 'vitest'

import type { SpendDay } from '../gen/bindings'
import { ago, dollars, heights, resetIn, tokens } from './usageFormat'

const day = (costUsd: number, input: number): SpendDay => ({
  day: '2026-09-14',
  costUsd,
  tokens: { input, output: 0, cacheRead: 0, cacheWrite: 0 },
  byModel: [],
})

describe('the usage screen', () => {
  it('says dollars to the cent and tokens at a glance', () => {
    expect(dollars(12.345)).toBe('$12.35')
    expect(dollars(0.004)).toBe('<$0.01')
    expect(dollars(0)).toBe('$0.00')
    expect(tokens(950)).toBe('950')
    expect(tokens(12_400)).toBe('12.4K')
    expect(tokens(3_100_000)).toBe('3.1M')
  })

  it('says when a window resets and how long ago a session ran', () => {
    const now = 1_000_000_000_000
    expect(resetIn(now / 1000 + 2 * 3600 + 14 * 60, now)).toBe('resets in 2h 14m')
    expect(resetIn(now / 1000 + 3 * 86_400 + 3600, now)).toBe('resets in 3d 1h')
    expect(resetIn(now / 1000 - 60, now)).toBe('resets now')
    expect(resetIn(null, now)).toBeNull()
    expect(ago(now / 1000 - 90 * 60, now)).toBe('1h ago')
  })

  it('draws each day against the tallest one, in the metric chosen', () => {
    expect(heights([day(1, 300), day(4, 100), day(0, 0)], 'cost')).toEqual([0.25, 1, 0])
    expect(heights([day(1, 300), day(4, 100)], 'tokens')).toEqual([1, 1 / 3])
    expect(heights([day(0, 0)], 'cost')).toEqual([0])
  })
})
