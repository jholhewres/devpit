import { describe, expect, it } from 'vitest'

import { COMPACT_FOR, EXPANDED_FOR, LEFT_FOR, due, next, resting, type Island, type Nudge } from './islandMachine'

const run = (nudges: [Nudge, number][], held = false, from: Island = resting(0)): Island =>
  nudges.reduce((island, [nudge, at]) => next(island, nudge, at, held), from)

describe('the island', () => {
  it('shows the pill when work happens and hides it after a quiet minute', () => {
    const shown = run([[{ kind: 'activity' }, 0]])
    expect(shown.mode).toBe('compact')
    expect(run([[{ kind: 'tick' }, COMPACT_FOR - 1]], false, shown).mode).toBe('compact')
    expect(run([[{ kind: 'tick' }, COMPACT_FOR]], false, shown).mode).toBe('hidden')
  })

  it('keeps the pill while something waits on the person', () => {
    const shown = run([[{ kind: 'activity' }, 0]], true)
    expect(run([[{ kind: 'tick' }, COMPACT_FOR * 5]], true, shown).mode).toBe('compact')
    expect(due(shown, true)).toBeNull()
  })

  it('opens all the way for an alert, and folds back to the pill on its own', () => {
    const opened = run([[{ kind: 'alert' }, 0]])
    expect(opened.mode).toBe('expanded')
    expect(run([[{ kind: 'tick' }, EXPANDED_FOR]], false, opened).mode).toBe('compact')
  })

  it('stays open while the cursor is on it, and counts from when it left', () => {
    const hovered = run([
      [{ kind: 'alert' }, 0],
      [{ kind: 'enter' }, 1_000],
    ])
    expect(run([[{ kind: 'tick' }, EXPANDED_FOR * 3]], false, hovered).mode).toBe('expanded')
    const left = run([[{ kind: 'leave' }, 50_000]], false, hovered)
    expect(run([[{ kind: 'tick' }, 50_000 + LEFT_FOR - 1]], false, left).mode).toBe('expanded')
    expect(run([[{ kind: 'tick' }, 50_000 + LEFT_FOR]], false, left).mode).toBe('compact')
  })

  /* Somebody who moved away from it, to click something else, is done with
     it; an island that opened on its own gives them time to look. */
  it('folds soon after the cursor leaves, and waits longer when it opened on its own', () => {
    const opened = run([[{ kind: 'alert' }, 0]])
    expect(opened.touched).toBe(false)
    expect(due(opened, false)).toBe(EXPANDED_FOR)
    const visited = run(
      [
        [{ kind: 'enter' }, 100],
        [{ kind: 'leave' }, 200],
      ],
      false,
      opened,
    )
    expect(due(visited, false)).toBe(200 + LEFT_FOR)
  })

  /* However it was opened: an island opened by something other than the
     cursor must not stay open waiting for a cursor that never came. */
  it('folds back on its own even when the cursor never passed over it', () => {
    const opened = run([[{ kind: 'open' }, 0]])
    expect(opened.hovering).toBe(false)
    expect(run([[{ kind: 'tick' }, EXPANDED_FOR]], false, opened).mode).toBe('compact')
  })

  it('peeks out of hiding when the cursor reaches the top', () => {
    expect(run([[{ kind: 'enter' }, 0]]).mode).toBe('compact')
  })

  it('does not fold while pinned, and closing unpins it', () => {
    const pinned = run([
      [{ kind: 'open' }, 0],
      [{ kind: 'pin', pinned: true }, 0],
    ])
    expect(run([[{ kind: 'tick' }, EXPANDED_FOR * 10]], false, pinned).mode).toBe('expanded')
    const closed = run([[{ kind: 'close' }, 1]], false, pinned)
    expect(closed.mode).toBe('compact')
    expect(closed.pinned).toBe(false)
  })

  it('does not shrink the open island when more work happens', () => {
    const opened = run([
      [{ kind: 'open' }, 0],
      [{ kind: 'activity' }, 10],
    ])
    expect(opened.mode).toBe('expanded')
  })
})

describe('a pause', () => {
  it('puts the island down, whatever it was doing', () => {
    const open = next(resting(0), { kind: 'alert' }, 0, true)
    expect(open.mode).toBe('expanded')
    expect(next({ ...open, pinned: true }, { kind: 'rest' }, 5, true).mode).toBe('hidden')
  })
})
