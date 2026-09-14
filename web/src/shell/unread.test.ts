import { act, renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import type { Tab } from './strip'
import { afterLooking, afterState, unreadIn, useUnread } from './unread'
import type { Doing } from './useAgents'

const none: ReadonlySet<string> = new Set()

describe('a finish nobody has looked at', () => {
  it('is unread when it arrives away from the tab in front', () => {
    expect(afterState(none, 'leaf_2', 'done', ['leaf_1']).has('leaf_2')).toBe(true)
  })

  it('is not news in the tab you were watching', () => {
    expect(afterState(none, 'leaf_1', 'done', ['leaf_1']).has('leaf_1')).toBe(false)
  })

  it('is read the moment its tab comes to the front', () => {
    const unread = afterState(none, 'leaf_2', 'done', ['leaf_1'])
    expect(afterLooking(unread, ['leaf_2']).has('leaf_2')).toBe(false)
    // Looking at another tab reads nothing.
    expect(afterLooking(unread, ['leaf_3']).has('leaf_2')).toBe(true)
  })

  it('stops being a finish once the agent works or waits again', () => {
    const unread = afterState(none, 'leaf_2', 'done', ['leaf_1'])
    expect(afterState(unread, 'leaf_2', 'working', ['leaf_1']).has('leaf_2')).toBe(false)
    expect(afterState(unread, 'leaf_2', 'waiting', ['leaf_1']).has('leaf_2')).toBe(false)
  })

  it('belongs to a split tab through either of its panes', () => {
    const tab: Tab = { id: 'term_1', kind: 'term', panes: ['leaf_1', 'leaf_2'] }
    expect(unreadIn(new Set(['leaf_2']), tab)).toBe(true)
    expect(unreadIn(new Set(['leaf_9']), tab)).toBe(false)
  })
})

describe('the shell keeping track of it', () => {
  const one: Tab = { id: 'term_1', kind: 'term', panes: ['leaf_1'] }
  const two: Tab = { id: 'term_2', kind: 'term', panes: ['leaf_2'] }

  it('holds a background finish until its tab is looked at', () => {
    const { result, rerender } = renderHook(
      ({ doing, active }: { doing: Doing; active: Tab }) => useUnread(doing, active),
      { initialProps: { doing: { leaf_2: 'working' } as Doing, active: one } },
    )
    act(() => rerender({ doing: { leaf_2: 'done' }, active: one }))
    expect(result.current.has('leaf_2')).toBe(true)

    act(() => rerender({ doing: { leaf_2: 'done' }, active: two }))
    expect(result.current.has('leaf_2')).toBe(false)
  })

  it('does not count what was already finished when the window opened', () => {
    const { result } = renderHook(() => useUnread({ leaf_2: 'done' }, one))
    expect(result.current.size).toBe(0)
  })
})
