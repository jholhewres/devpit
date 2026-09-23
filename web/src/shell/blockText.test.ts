import { describe, expect, it } from 'vitest'

import type { CommandBlock } from '../gen/bindings'
import { filtered, outcome, shortPath, took } from './blockText'

const block = (over: Partial<CommandBlock>): CommandBlock => ({
  id: 1, command: 'x', cwd: null, startedAt: 0, endedAt: 0, code: 0, interactive: false, truncated: false, ...over,
})

describe('a block’s header', () => {
  it('says how long it took in the fewest words', () => {
    expect(took(block({ endedAt: 320 }), 0)).toBe('320ms')
    expect(took(block({ endedAt: 4200 }), 0)).toBe('4.2s')
    expect(took(block({ endedAt: 185_000 }), 0)).toBe('3m 05s')
    expect(took(block({ endedAt: null }), 2000)).toBe('2.0s')
  })

  it('writes home as ~', () => {
    expect(shortPath('/home/j/work', '/home/j')).toBe('~/work')
    expect(shortPath('/home/jo', '/home/j')).toBe('/home/jo')
  })

  it('colours by how it ended', () => {
    expect(outcome(block({ code: 0 }))).toBe('ok')
    expect(outcome(block({ code: 2 }))).toBe('failed')
    expect(outcome(block({ endedAt: null }))).toBe('running')
    expect(outcome(block({ code: null }))).toBe('unknown')
  })

  it('filters output lines by every word', () => {
    expect(filtered(['error: a', 'ok', 'Error b'], 'error')).toEqual([0, 2])
    expect(filtered(['a', 'b'], '')).toEqual([0, 1])
  })
})
