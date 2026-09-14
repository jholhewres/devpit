import { describe, expect, it } from 'vitest'

import { draggingAfter } from './window'

describe('whether files are hovering over the window', () => {
  it('starts on enter and holds while they move', () => {
    expect(draggingAfter('enter')).toBe(true)
    expect(draggingAfter('over')).toBe(true)
  })

  it('ends on a drop and on a leave alike', () => {
    // A drop that left the target drawn would sit over the chat until the
    // next drag came along to clear it.
    expect(draggingAfter('drop')).toBe(false)
    expect(draggingAfter('leave')).toBe(false)
  })
})
