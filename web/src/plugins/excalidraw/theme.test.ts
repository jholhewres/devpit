import { describe, expect, it } from 'vitest'

import { excalidrawTheme } from './theme'

describe('excalidrawTheme', () => {
  it('maps light straight through, regardless of the system', () => {
    expect(excalidrawTheme('light', true)).toBe('light')
    expect(excalidrawTheme('light', false)).toBe('light')
  })

  it('maps dark straight through, regardless of the system', () => {
    expect(excalidrawTheme('dark', true)).toBe('dark')
    expect(excalidrawTheme('dark', false)).toBe('dark')
  })

  it('borrows the system when the preference is system', () => {
    expect(excalidrawTheme('system', true)).toBe('dark')
    expect(excalidrawTheme('system', false)).toBe('light')
  })
})
