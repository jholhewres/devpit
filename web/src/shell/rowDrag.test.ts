import { describe, expect, it } from 'vitest'

import { canDropOn, destinationIn, edgeScrollDelta } from './rowDrag'

describe('whether a drop is allowed', () => {
  it('refuses a folder dropped on itself', () => {
    expect(canDropOn('src', 'src')).toBe(false)
  })

  it('refuses a folder dropped on its own child', () => {
    expect(canDropOn('src', 'src/lib')).toBe(false)
  })

  it('refuses a folder dropped several levels into its own descendant', () => {
    expect(canDropOn('src', 'src/lib/deep')).toBe(false)
  })

  it('does not mistake a sibling with a shared prefix for a descendant', () => {
    // "src-old" is not inside "src" — the prefix check must require the slash.
    expect(canDropOn('src', 'src-old')).toBe(true)
  })

  it('allows a folder dropped on an unrelated folder', () => {
    expect(canDropOn('src', 'lib')).toBe(true)
  })

  it('allows a file dropped on any folder other than itself', () => {
    expect(canDropOn('src/main.rs', 'lib')).toBe(true)
  })

  /* The paths differ, so the self/descendant checks above let this through
     — but the move is a no-op the backend answers with `AlreadyExists`,
     which is worse than doing nothing. */
  it('refuses a file dropped back onto its own parent folder', () => {
    expect(canDropOn('a/b.txt', 'a')).toBe(false)
  })

  it('refuses a folder dropped back onto its own parent folder', () => {
    expect(canDropOn('a/b', 'a')).toBe(false)
  })
})

describe('where a dropped path lands', () => {
  it('joins the dragged name under the target folder', () => {
    expect(destinationIn('src/main.rs', 'lib')).toBe('lib/main.rs')
  })

  it('keeps only the last segment of a deeper dragged path', () => {
    expect(destinationIn('a/b/c.rs', 'lib')).toBe('lib/c.rs')
  })

  it('drops at the root when the target folder is the root', () => {
    expect(destinationIn('a/b/c.rs', '')).toBe('c.rs')
  })
})

describe('nudging the scroll container near a drag', () => {
  const rect = { top: 100, bottom: 300 }

  it('does nothing away from either edge', () => {
    expect(edgeScrollDelta(rect, 200)).toBe(0)
  })

  it('scrolls up near the top edge', () => {
    expect(edgeScrollDelta(rect, 105)).toBeLessThan(0)
  })

  it('scrolls down near the bottom edge', () => {
    expect(edgeScrollDelta(rect, 295)).toBeGreaterThan(0)
  })
})
