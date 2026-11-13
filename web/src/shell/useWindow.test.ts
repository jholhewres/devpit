import { describe, expect, it } from 'vitest'

import { MOST, rangeOf, spacers, spanAt, type Spacers } from './useWindow'


describe("a long thread's window", () => {
  it('draws the rows in view and a margin each side, no more', () => {
    // 2,000 rows of 100 px, the view 1,000 px tall at 50,000 px down, 300 px each side.
    const heights = Array.from({ length: 2000 }, () => 100)
    expect(spanAt(heights, 50_000, 1000, 300)).toEqual({ from: 497, to: 513 })
  })

  it('never draws more than its most, however tall the view', () => {
    const heights = Array.from({ length: 2000 }, () => 10)
    const span = spanAt(heights, 0, 5000, 600)
    expect(span.to - span.from).toBe(MOST)
  })

  it('keeps the rows being read when a message arrives at the end', () => {
    const before = ['a', 'b', 'c', 'd']
    const after = [...before, 'e']
    const span = { tail: false, first: 'b', count: 2 }
    expect(rangeOf(after, span)).toEqual(rangeOf(before, span))
  })

  it('follows the end while it is drawn, taking the message that arrives', () => {
    const after = ['a', 'b', 'c', 'd', 'e']
    expect(rangeOf(after, { tail: true, first: null, count: 2 })).toEqual({ from: 3, to: 5 })
  })
})

describe("a long thread's spacers", () => {
  const sum = (heights: readonly number[], start: number, end: number): number => heights.slice(start, end).reduce((total, one) => total + one, 0)

  it('stay what a sum of every row above and below the window says, as the window moves and messages arrive', () => {
    const heights: number[] = Array.from({ length: 500 }, (_, at) => 40 + ((at * 37) % 90))
    let ids = heights.map((_, at) => `m${at}`)
    let kept: Spacers | null = null
    // Down the thread, back up, and to the end while three messages arrive.
    const moves: [number, number, number][] = [[0, 20, 0], [30, 70, 0], [100, 140, 0], [90, 130, 0], [5, 40, 0], [460, 500, 0], [461, 501, 1], [463, 503, 2]]
    for (const [from, to, arrived] of moves) {
      for (let at = 0; at < arrived; at += 1) heights.push(55 + at)
      ids = heights.map((_, at) => `m${at}`)
      kept = spacers(kept, ids, from, to, (at) => heights[at]!)
      expect([kept.before, kept.after]).toEqual([sum(heights, 0, from), sum(heights, to, heights.length)])
    }
  })

  it('read no row again for a draw that moved nothing', () => {
    const ids = Array.from({ length: 2000 }, (_, at) => `m${at}`)
    const first = spacers(null, ids, 1960, 2000, () => 100)
    let read = 0
    const again = spacers(first, ids, 1960, 2000, () => (read++, 100))
    expect(read).toBe(0)
    expect([again.before, again.after]).toEqual([196_000, 0])
  })
})
