import { describe, expect, it } from 'vitest'

import { movesDisk } from './diskMoves'

const said = (what: string, detail: string | null = null) => ({ paneId: 'leaf_1', what, detail })

describe('movesDisk', () => {
  it('counts a command ending and an agent stopping', () => {
    expect(movesDisk(said('finished', '0'))).toBe(true)
    expect(movesDisk(said('agent', 'done'))).toBe(true)
    expect(movesDisk(said('agent', 'waiting'))).toBe(true)
  })

  it('leaves out what does not touch the checkout', () => {
    expect(movesDisk(said('agent', 'working'))).toBe(false)
    expect(movesDisk(said('cwd', '/w'))).toBe(false)
    expect(movesDisk(said('prompt'))).toBe(false)
  })
})
