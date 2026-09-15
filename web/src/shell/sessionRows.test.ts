import { describe, expect, it } from 'vitest'

import { adoptable } from './sessionRows'

describe('what Open as chat may take', () => {
  it('takes a run once it is not running', () => {
    expect(adoptable('run', 'working')).toBe('stop-first')
    expect(adoptable('run', 'done')).toBe('adopt')
    expect(adoptable('run', 'failed')).toBe('adopt')
    expect(adoptable('run', null)).toBe('adopt')
  })

  it('takes a background session only once it has finished or gone', () => {
    expect(adoptable('background', 'done')).toBe('adopt')
    expect(adoptable('background', 'gone')).toBe('adopt')
    for (const state of ['open', 'working', 'waiting', null] as const) {
      expect(adoptable('background', state)).toBe('stop-first')
    }
  })

  it('leaves a pane to its terminal and a chat as it is', () => {
    expect(adoptable('pane', 'waiting')).toBe('in-terminal')
    expect(adoptable('chat', 'done')).toBe('chat')
  })
})
