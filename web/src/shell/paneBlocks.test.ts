import { describe, expect, it } from 'vitest'

import type { CommandBlock } from '../gen/bindings'
import { changed, EMPTY, finished, happened, jumpTarget, modeOf, runningOf } from './paneBlocks'

const block = (id: number, over: Partial<CommandBlock> = {}): CommandBlock => ({
  id,
  command: `cmd ${id}`,
  cwd: '/w',
  startedAt: id,
  endedAt: null,
  code: null,
  interactive: false,
  truncated: false,
  bookmarked: false,
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

  it('jumps between bookmarked blocks, or between every block when none is', () => {
    const plain = [block(1), block(2), block(3)]
    expect(jumpTarget(plain, null, -1)).toBe(3)
    expect(jumpTarget(plain, 3, -1)).toBe(2)
    expect(jumpTarget(plain, 1, -1)).toBe(1)
    expect(jumpTarget(plain, 2, 1)).toBe(3)
    expect(jumpTarget(plain, 3, 1)).toBeNull()
    expect(jumpTarget(plain, null, 1)).toBeNull()

    const marked = [block(1, { bookmarked: true }), block(2), block(3, { bookmarked: true }), block(4)]
    expect(jumpTarget(marked, null, -1)).toBe(3)
    expect(jumpTarget(marked, 3, -1)).toBe(1)
    expect(jumpTarget(marked, 1, 1)).toBe(3)
    expect(jumpTarget(marked, 3, 1)).toBeNull()
    /* From a block that was cleared away: as from the bottom. */
    expect(jumpTarget(marked, 99, -1)).toBe(3)
  })
})
