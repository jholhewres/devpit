import { describe, expect, it } from 'vitest'

import { leading } from './RemoteRow'

describe('leading', () => {
  it('leads with what brings the two sides together', () => {
    expect(leading(0, 3)).toBe('pull')
    expect(leading(2, 0)).toBe('push')
    expect(leading(2, 3)).toBe('sync')
    expect(leading(0, 0)).toBe('sync')
  })
})
