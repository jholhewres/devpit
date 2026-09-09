import { describe, expect, it } from 'vitest'

import { bytes } from './disk'

describe('how much room something takes', () => {
  it('says nothing when nothing could be read', () => {
    expect(bytes(null)).toBeNull()
  })

  it('says nothing for an empty folder rather than drawing a zero', () => {
    expect(bytes(0)).toBeNull()
  })

  it('keeps whole numbers up to megabytes', () => {
    expect(bytes(2048)).toBe('2 KB')
  })

  it('gives one decimal from megabytes up, which is what a person acts on', () => {
    expect(bytes(1503238553)).toBe('1.4 GB')
  })
})
