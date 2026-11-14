import { describe, expect, it, vi } from 'vitest'

import { sessionMenu } from './sessionMenu'
import { NO_MARKS } from './useConversations'

const hands = () => ({ focus: vi.fn(), close: vi.fn(), rename: vi.fn(), mark: vi.fn() })
const labels = (entries: ReturnType<typeof sessionMenu>): string[] => entries.flatMap((one) => (one.label ? [one.label] : []))
const EVENING = new Date(2026, 10, 13, 21, 30)

describe('a session row’s menu', () => {
  it('offers pin, settle, snooze and mark unread on a conversation that was spoken in', () => {
    expect(labels(sessionMenu(hands(), NO_MARKS, EVENING))).toEqual([
      'Open',
      'Rename',
      'Pin',
      'Settle',
      'Snooze for an hour',
      'Snooze until tomorrow',
      'Mark unread',
      'Close session',
    ])
    expect(labels(sessionMenu(hands()))).toEqual(['Open', 'Rename', 'Close session'])
  })

  it('keeps each change on the conversation', () => {
    const held = hands()
    const entries = sessionMenu(held, NO_MARKS, EVENING)
    const run = (label: string): void => entries.find((one) => one.label === label)?.run?.('c1')
    run('Pin')
    run('Settle')
    run('Snooze until tomorrow')
    run('Mark unread')
    expect(held.mark.mock.calls).toEqual([
      ['c1', { pinned: true }],
      ['c1', { settled: true }],
      ['c1', { snoozedUntil: new Date(2026, 10, 14, 9).getTime() / 1000 }],
      ['c1', { unread: true }],
    ])
  })

  it('brings a settled or snoozed one back to in progress, and unpins a pinned one', () => {
    const held = hands()
    const snoozed = sessionMenu(held, { ...NO_MARKS, pinned: true, snoozedUntil: EVENING.getTime() / 1000 + 60 }, EVENING)
    expect(labels(snoozed)).toContain('Unpin')
    snoozed.find((one) => one.label === 'Back to in progress')?.run?.('c1')
    expect(held.mark).toHaveBeenCalledWith('c1', { settled: false, snoozedUntil: null })
  })
})
