import { afterEach, describe, expect, it, vi } from 'vitest'

/* One Tauri listener per event, whatever the number of subscribers. */
const handlers: Array<(event: { payload: unknown }) => void> = []
const unlistened = vi.fn()
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((_name: string, handler: (event: { payload: unknown }) => void) => {
    handlers.push(handler)
    return Promise.resolve(unlistened)
  }),
}))

afterEach(() => {
  delete (window as unknown as { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__
  handlers.length = 0
  unlistened.mockClear()
})

describe('events heard by many parts of the window', () => {
  it('cross once, reach every subscriber, and stop with the last one', async () => {
    ;(window as unknown as { __TAURI_INTERNALS__: object }).__TAURI_INTERNALS__ = {}
    const { onCarried } = await import('./window')
    const one = vi.fn()
    const two = vi.fn()
    const dropOne = onCarried<string>('run:changed', one)
    const dropTwo = onCarried<string>('run:changed', two)
    await Promise.resolve()
    expect(handlers).toHaveLength(1)

    handlers[0]!({ payload: 'card_1' })
    expect(one).toHaveBeenCalledWith('card_1')
    expect(two).toHaveBeenCalledWith('card_1')

    dropOne()
    expect(unlistened).not.toHaveBeenCalled()
    dropTwo()
    expect(unlistened).toHaveBeenCalledOnce()
  })
})
