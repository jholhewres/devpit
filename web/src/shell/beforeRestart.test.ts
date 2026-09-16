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
  it('flushes what is holding on, waits for the write, then says it is ready', async () => {
    const saved: string[] = []
    let written = (): void => {}
    const stopWatching = answerBeforeRestart()
    const stopHolding = savesBeforeRestart(
      () =>
        new Promise<void>((done) => {
          saved.push('a card')
          written = done
        }),
    )

    heard['update:before-restart']!(null)
    await vi.waitFor(() => expect(saved).toEqual(['a card']))
    expect(ready).not.toHaveBeenCalled()

    written()
    await vi.waitFor(() => expect(ready).toHaveBeenCalled())
    stopHolding()
    stopWatching()
  })

  /* One editor throwing must not leave the update waiting out its deadline. */
  it('says it is ready even when a save throws', async () => {
    const stopWatching = answerBeforeRestart()
    const stopHolding = savesBeforeRestart(() => {
      throw new Error('no')
    })

    heard['update:before-restart']!(null)

    await vi.waitFor(() => expect(ready).toHaveBeenCalled())
    stopHolding()
    stopWatching()
  })
})
