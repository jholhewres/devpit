import { describe, expect, it } from 'vitest'

import { closed, moved, opened, type Strip } from './strip'

const strip = (open: string[], active: string | null = open[0] ?? null): Strip =>
  ({ open, active }) as Strip

describe('opening a pane', () => {
  it('appends it and looks at it', () => {
    expect(opened(strip([]), 'board' as never)).toEqual({ open: ['board'], active: 'board' })
  })

  it('does not move one that is already open', () => {
    /* Clicking a tab must not reshuffle the row under the pointer. */
    const before = strip(['board', 'term', 'chat'] as never[], 'chat' as never)
    expect(opened(before, 'board' as never).open).toEqual(['board', 'term', 'chat'])
  })
})

describe('closing a pane', () => {
  it('lands on the neighbour when it was the active one', () => {
    const before = strip(['board', 'term', 'chat'] as never[], 'term' as never)
    expect(closed(before, 'term' as never)).toEqual({ open: ['board', 'chat'], active: 'chat' })
  })

  it('leaves you where you were when it was not', () => {
    const before = strip(['board', 'term', 'chat'] as never[], 'chat' as never)
    expect(closed(before, 'board' as never).active).toBe('chat')
  })

  it('empties the strip when the last one goes', () => {
    /* This is the case that shipped broken in the prototype: closing the only
       tab threw before the redraw, so the window froze on a pane that was
       already gone from the state. */
    expect(closed(strip(['board'] as never[]), 'board' as never)).toEqual({
      open: [],
      active: null,
    })
  })

  it('ignores a pane that is not open', () => {
    const before = strip(['board'] as never[])
    expect(closed(before, 'term' as never)).toBe(before)
  })
})

describe('dragging a tab', () => {
  it('puts it where it was dropped', () => {
    const before = strip(['board', 'term', 'chat'] as never[])
    expect(moved(before, 'chat' as never, 0).open).toEqual(['chat', 'board', 'term'])
  })

  it('refuses a landing spot outside the strip', () => {
    const before = strip(['board', 'term'] as never[])
    expect(moved(before, 'board' as never, 5)).toBe(before)
  })
})
