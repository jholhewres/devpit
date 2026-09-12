import { describe, expect, it } from 'vitest'

import { reordered } from './laneOrder'

const lanes = ['a', 'b', 'c', 'd']

describe('reordering the columns', () => {
  it('moves one to the left', () => {
    expect(reordered(lanes, 'c', 'a')).toEqual(['c', 'a', 'b', 'd'])
  })

  /* The case the naive version gets wrong: taking the column out shifts
     everything after it down one, so inserting at the original index lands a
     place short of where the pointer is. */
  it('moves one to the right, past the gap it left', () => {
    expect(reordered(lanes, 'a', 'c')).toEqual(['b', 'c', 'a', 'd'])
    expect(reordered(lanes, 'a', 'd')).toEqual(['b', 'c', 'd', 'a'])
  })

  it('is the same list when a column is dropped on itself', () => {
    expect(reordered(lanes, 'b', 'b')).toBe(lanes)
  })

  it('is the same list for an id that is not in it', () => {
    expect(reordered(lanes, 'z', 'a')).toBe(lanes)
    expect(reordered(lanes, 'a', 'z')).toBe(lanes)
  })

  it('keeps every column, and only moves one', () => {
    const after = reordered(lanes, 'b', 'd')
    expect([...after].sort()).toEqual([...lanes].sort())
    expect(after).toHaveLength(lanes.length)
  })
})
