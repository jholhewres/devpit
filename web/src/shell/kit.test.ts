import { describe, expect, it } from 'vitest'

import { counted } from './useKit'

/* The badge used to read `5` and `2/4` from nowhere. A live count only helps
   if it stays quiet when it has read nothing: `?? 0` puts the same lie back,
   in a number that moves. */
describe('counted', () => {
  it('says nothing when the list could not be read', () => {
    expect(counted(undefined)).toBeNull()
  })

  it('says zero when it read the list and the list was empty', () => {
    expect(counted([])).toBe(0)
  })

  it('counts what it read', () => {
    expect(counted(['claude', 'codex'])).toBe(2)
  })
})
