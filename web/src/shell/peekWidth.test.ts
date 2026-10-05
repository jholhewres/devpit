import { describe, expect, it } from 'vitest'

import { PEEK_DEFAULT, peekWidth } from './peekWidth'

describe('how wide a document beside the chat is', () => {
  it('is what was pulled to, within room for both', () => {
    expect(peekWidth(600, 1400)).toBe(600)
    // Never so narrow it cannot be read, nor so wide the chat goes.
    expect(peekWidth(100, 1400)).toBe(320)
    expect(peekWidth(1300, 1400)).toBe(1040)
  })

  it('keeps its least in a pane too narrow for both', () => {
    expect(peekWidth(PEEK_DEFAULT, 500)).toBe(320)
  })
})
