import { describe, expect, it } from 'vitest'

import { backoff, base64Of, bytesOf } from './socket'

describe("the remote viewer's link", () => {
  it('waits longer after each failed try, and never past a ceiling', () => {
    expect(backoff(0)).toBeGreaterThanOrEqual(500)
    expect(backoff(1)).toBeGreaterThanOrEqual(1000)
    expect(backoff(40)).toBeLessThanOrEqual(15_250)
  })

  it('carries keys as base64, accents and control keys alike', () => {
    for (const keys of ['ls -la\r', '\x1b[A', 'ação', '\x03']) {
      expect(new TextDecoder().decode(bytesOf(base64Of(keys)))).toBe(keys)
    }
  })
})
