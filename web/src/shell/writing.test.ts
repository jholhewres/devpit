import { describe, expect, it } from 'vitest'

import { typed, wordEnd } from './writing'

const live = { live: true, reduced: false }
const words = 'Rivers carve valleys through stone over many years and they carry silt to the sea. '.repeat(20)

describe('the pace an answer is written at', () => {
  it('types steadily from a burst instead of showing it at once', () => {
    const after = typed(words, 0, 33, live)
    expect(after).toBeGreaterThan(0)
    expect(after).toBeLessThan(words.length / 4)
  })

  it('catches up with a burst within a second and a half', () => {
    let shown = 0
    for (let ms = 0; ms < 1600; ms += 33) shown = typed(words, shown, 33, live)
    expect(shown).toBe(words.length)
  })

  it('never stops slower than its floor, and never shows past what arrived', () => {
    expect(typed('a few words', 0, 100, live)).toBeGreaterThanOrEqual(8)
    expect(typed('short', 0, 5000, live)).toBe(5)
  })

  it('shows everything when the turn is over, or when motion is reduced', () => {
    expect(typed(words, 3, 1, { live: false, reduced: false })).toBe(words.length)
    expect(typed(words, 3, 1, { live: true, reduced: true })).toBe(words.length)
  })

  it('stops at the end of a word, not in the middle of one', () => {
    expect(wordEnd('carve valleys', 2)).toBe(5)
    expect(wordEnd('carve valleys', 5)).toBe(5)
    expect(wordEnd('x'.repeat(40), 3)).toBe(3)
  })
})
