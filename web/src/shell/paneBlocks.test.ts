import { describe, expect, it } from 'vitest'

import type { CommandBlock } from '../gen/bindings'
import { changed, EMPTY, finished, happened, modeOf, runningOf } from './paneBlocks'

const block = (id: number, over: Partial<CommandBlock> = {}): CommandBlock => ({
  id,
  command: `cmd ${id}`,
  cwd: '/w',
  startedAt: id,
  endedAt: null,
  code: null,
  interactive: false,
  truncated: false,
  ...over,
})

describe('a terminal’s blocks', () => {
  it('is the terminal as it was until the shell is heard marking its prompts', () => {
    expect(modeOf(EMPTY, true)).toBe('classic')
    const prompt = happened(EMPTY, { paneId: 'p', what: 'prompt', detail: null })
    expect(modeOf(prompt, true)).toBe('idle')
    expect(modeOf(prompt, false)).toBe('classic')
  })

  it('runs a command in place, then keeps it as a block', () => {
    let state = happened(EMPTY, { paneId: 'p', what: 'prompt', detail: null })
    state = changed(state, { paneId: 'p', block: block(1) })
    expect(modeOf(state, true)).toBe('running')
    expect(runningOf(state)?.id).toBe(1)
    state = changed(state, { paneId: 'p', block: block(1, { endedAt: 5, code: 0 }) })
    state = happened(state, { paneId: 'p', what: 'prompt', detail: null })
    expect(modeOf(state, true)).toBe('idle')
    expect(finished(state).map((one) => one.id)).toEqual([1])
    expect(state.blocks).toHaveLength(1)
  })

  it('gives a full-screen command the whole terminal', () => {
    const state = changed(happened(EMPTY, { paneId: 'p', what: 'prompt', detail: null }), {
      paneId: 'p',
      block: block(2, { interactive: true }),
    })
    expect(modeOf(state, true)).toBe('full')
  })

  it('keeps the folder the shell last named', () => {
    expect(happened(EMPTY, { paneId: 'p', what: 'cwd', detail: '/home' }).cwd).toBe('/home')
  })
})
