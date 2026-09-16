import { afterEach, describe, expect, it, vi } from 'vitest'

import { answerBeforeRestart, savesBeforeRestart } from './beforeRestart'

const heard: Record<string, (payload: null) => void> = {}
vi.mock('./window', () => ({
  onCarried: (name: string, then: (payload: null) => void) => {
    heard[name] = then
    return () => delete heard[name]
  },
}))

const ready = vi.fn(async () => ({ status: 'ok', data: null }))
vi.mock('./live', () => ({ commands: { updateRestartReady: () => ready() } }))

afterEach(() => vi.clearAllMocks())

describe('what the window puts down before a restart', () => {
  it('flushes what is holding on, then says it is ready', () => {
    const saved: string[] = []
    const stopWatching = answerBeforeRestart()
    const stopHolding = savesBeforeRestart(() => saved.push('a card'))

    heard['update:before-restart']!(null)

    expect(saved).toEqual(['a card'])
    expect(ready).toHaveBeenCalled()
    stopHolding()
    stopWatching()
  })

  /* One editor throwing must not leave the update waiting out its deadline. */
  it('says it is ready even when a save throws', () => {
    const stopWatching = answerBeforeRestart()
    const stopHolding = savesBeforeRestart(() => {
      throw new Error('no')
    })

    heard['update:before-restart']!(null)

    expect(ready).toHaveBeenCalled()
    stopHolding()
    stopWatching()
  })
})
