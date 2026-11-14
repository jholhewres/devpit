import { describe, expect, it } from 'vitest'

import type { ConversationMarks } from '../gen/bindings'
import { grouped, snoozeUntil } from './settling'

const NONE: ConversationMarks = { name: null, pinned: false, settled: false, snoozedUntil: null, unread: false }
const NOW = 1_800_000_000

describe('where a conversation sits', () => {
  it('splits in progress from settled and snoozed, pinned first in each', () => {
    const marks: Record<string, ConversationMarks> = {
      a: NONE,
      b: { ...NONE, pinned: true },
      c: { ...NONE, settled: true },
      d: { ...NONE, snoozedUntil: NOW + 60 },
      e: { ...NONE, snoozedUntil: NOW - 60 },
      f: { ...NONE, settled: true, pinned: true },
    }
    const { progress, settled } = grouped(['a', 'b', 'c', 'd', 'e', 'f'], (id) => marks[id], NOW)
    expect(progress).toEqual(['b', 'a', 'e'])
    expect(settled).toEqual(['f', 'c', 'd'])
  })

  it('leaves a row with no marks in progress', () => {
    expect(grouped(['x'], () => undefined, NOW)).toEqual({ progress: ['x'], settled: [] })
  })

  it('snoozes for an hour, or until nine tomorrow morning', () => {
    const evening = new Date(2026, 10, 13, 21, 30)
    expect(snoozeUntil('hour', evening)).toBe(Math.floor(evening.getTime() / 1000) + 3600)
    expect(new Date(snoozeUntil('tomorrow', evening) * 1000)).toEqual(new Date(2026, 10, 14, 9))
  })
})
