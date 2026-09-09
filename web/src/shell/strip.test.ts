import { describe, expect, it } from 'vitest'

import { closed, focused, many, moved, opened, renamed, type Strip, type Tab } from './strip'
import { empty } from './tabs'

const tab = (id: string, kind = 'term'): Tab => ({ id, kind: kind as Tab['kind'] })
const strip = (tabs: Tab[], active: string | null = tabs[0]?.id ?? null): Strip =>
  ({ open: tabs, active })

describe('which kinds you can have several of', () => {
  it('lets you open many terminals and chats', () => {
    expect(many('term')).toBe(true)
    expect(many('chat')).toBe(true)
  })

  it('keeps one board', () => {
    expect(many('board')).toBe(false)
  })
})

describe('opening a tab', () => {
  it('appends it and looks at it', () => {
    expect(opened(strip([]), tab('t1'))).toEqual({ open: [tab('t1')], active: 't1' })
  })

  it('opens a second terminal rather than focusing the first', () => {
    const after = opened(strip([tab('t1')]), tab('t2'))
    expect(after.open.map((t) => t.id)).toEqual(['t1', 't2'])
    expect(after.active).toBe('t2')
  })

  it('focuses the board that is already open instead of adding another', () => {
    const before = strip([tab('b1', 'board'), tab('t1')], 't1')
    const after = opened(before, tab('b2', 'board'))
    expect(after.open).toHaveLength(2)
    expect(after.active).toBe('b1')
  })
})

describe('closing a tab', () => {
  it('lands on the neighbour when it was the active one', () => {
    const after = closed(strip([tab('a'), tab('b'), tab('c')], 'b'), 'b')
    expect(after.active).toBe('c')
  })

  it('leaves you where you were when it was not', () => {
    expect(closed(strip([tab('a'), tab('b')], 'b'), 'a').active).toBe('b')
  })

  it('empties the strip when the last one goes', () => {
    expect(closed(strip([tab('a')]), 'a')).toEqual({ open: [], active: null })
  })

  it('ignores a tab that is not open', () => {
    const before = strip([tab('a')])
    expect(closed(before, 'zz')).toBe(before)
  })
})

describe('dragging a tab', () => {
  it('puts it where it was dropped', () => {
    const after = moved(strip([tab('a'), tab('b'), tab('c')]), 'c', 0)
    expect(after.open.map((t) => t.id)).toEqual(['c', 'a', 'b'])
  })

  it('refuses a landing spot outside the strip', () => {
    const before = strip([tab('a'), tab('b')])
    expect(moved(before, 'a', 5)).toBe(before)
  })
})

describe('what a tab says', () => {
  it('takes the title the terminal reports', () => {
    const after = renamed(strip([tab('a')]), 'a', '~/devpit')
    expect(after.open[0]!.title).toBe('~/devpit')
  })

  it('names the one you are looking at', () => {
    expect(focused(strip([tab('a'), tab('b')], 'b'))?.id).toBe('b')
  })
})

describe('a file is one tab', () => {
  const file = (path: string): Tab => ({ id: `file:${path}`, kind: 'file', path })

  it('opens a second file beside the first', () => {
    const one = opened(empty, file('a.rs'))
    const two = opened(one, file('b.rs'))
    expect(two.open).toHaveLength(2)
    expect(two.active).toBe('file:b.rs')
  })

  it('focuses the tab it already has rather than opening the file twice', () => {
    const one = opened(opened(empty, file('a.rs')), file('b.rs'))
    const again = opened(one, file('a.rs'))
    expect(again.open).toHaveLength(2)
    expect(again.active).toBe('file:a.rs')
  })
})
