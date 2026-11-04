import { describe, expect, it } from 'vitest'

import type { Message } from '../gen/bindings'
import { MOST, rangeOf, spanAt } from './useWindow'

const message = (id: string): Message => ({ id, turnId: null, role: 'assistant', parts: [], createdAt: 0, streaming: false }) as Message

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
    const before = ['a', 'b', 'c', 'd'].map(message)
    const after = [...before, message('e')]
    const span = { tail: false, first: 'b', count: 2 }
    expect(rangeOf(after, span)).toEqual(rangeOf(before, span))
  })

  it('follows the end while it is drawn, taking the message that arrives', () => {
    const after = ['a', 'b', 'c', 'd', 'e'].map(message)
    expect(rangeOf(after, { tail: true, first: null, count: 2 })).toEqual({ from: 3, to: 5 })
  })
})
