import { act, renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import type { Tab } from './strip'
import { afterFinish, afterLooking, afterState, unreadIn, useUnread } from './unread'
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
    expect(result.current.unread.has('leaf_2')).toBe(true)

    act(() => rerender({ doing: { leaf_2: 'done' }, active: two }))
    expect(result.current.unread.has('leaf_2')).toBe(false)
  })

  it('does not count what was already finished when the window opened', () => {
    const { result } = renderHook(() => useUnread({ leaf_2: 'done' }, one))
    expect(result.current.unread.size).toBe(0)
  })
})

describe('afterState when nothing changes', () => {
  it('hands back the same set, so the shell does not re-render', () => {
    const some = new Set(['leaf_2'])
    expect(afterState(some, 'leaf_2', 'done', [])).toBe(some)
    expect(afterState(some, 'leaf_3', 'working', [])).toBe(some)
    expect(afterState(some, 'leaf_2', 'done', ['leaf_2'])).toBe(some)
  })
})

describe('a chat turn that ended', () => {
  const chat: Tab = { id: 'chat_1', kind: 'chat' }
  const term: Tab = { id: 'term_1', kind: 'term', panes: ['leaf_1'] }

  it('is news only away from the tab in front', () => {
    expect(afterFinish(new Set(), 'chat_1', ['leaf_1', 'term_1']).has('chat_1')).toBe(true)
    expect(afterFinish(new Set(), 'chat_1', ['chat_1']).size).toBe(0)
    expect(unreadIn(new Set(['chat_1']), chat)).toBe(true)
  })

  it('marks its tab while another is in front, and the mark goes when it is opened', () => {
    const { result, rerender } = renderHook(({ active }: { active: Tab }) => useUnread({}, active), { initialProps: { active: term } })
    act(() => result.current.finished('chat_1'))
    expect(unreadIn(result.current.unread, chat)).toBe(true)

    act(() => rerender({ active: chat }))
    expect(unreadIn(result.current.unread, chat)).toBe(false)
  })

  it('leaves no mark when it ends in the tab in front', () => {
    const { result } = renderHook(() => useUnread({}, chat))
    act(() => result.current.finished('chat_1'))
    expect(result.current.unread.size).toBe(0)
  })
})
