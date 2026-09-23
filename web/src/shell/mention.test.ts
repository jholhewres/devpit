import { describe, expect, it } from 'vitest'

import { mentionAt, mentioned } from './mention'

describe('@file in the composer', () => {
  it('reads the @word the caret ends', () => {
    expect(mentionAt('look at @src/ma', 15)).toEqual({ start: 8, query: 'src/ma' })
    expect(mentionAt('@', 1)).toEqual({ start: 0, query: '' })
  })

  it('leaves an address, and a finished mention, alone', () => {
    expect(mentionAt('mail me@host', 12)).toBeNull()
    expect(mentionAt('@a.rs and', 9)).toBeNull()
  })

  it('puts the path in its place, with the sentence going on after it', () => {
    const at = mentionAt('fix @ma now', 7)!
    expect(mentioned('fix @ma now', at, 7, 'src/main.rs')).toEqual({ text: 'fix @src/main.rs now', caret: 17 })
  })
})
