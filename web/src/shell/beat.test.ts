import { afterEach, describe, expect, it, vi } from 'vitest'

import { BEAT_MS, beat, doing, noteCards, noteView } from './beat'

afterEach(() => {
  vi.useRealTimers()
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
})

/* A page whose visibility the test sets. */
function page(): Document & { visibilityState: DocumentVisibilityState } {
  const target = new EventTarget() as Document & { visibilityState: DocumentVisibilityState }
  target.visibilityState = 'visible'
  return target
}

describe('the beat', () => {
  it('says the page is alive at once and every few seconds while it is shown, and stops when it is hidden', () => {
    ;(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {}
    vi.useFakeTimers()
    const shown = page()
    const send = vi.fn((_shown: boolean, _on: string) => Promise.resolve())
    const stop = beat(shown, send, BEAT_MS, () => 'view board')

    expect(send).toHaveBeenLastCalledWith(true, 'view board')
    vi.advanceTimersByTime(BEAT_MS * 2)
    expect(send).toHaveBeenCalledTimes(3)

    shown.visibilityState = 'hidden'
    shown.dispatchEvent(new Event('visibilitychange'))
    expect(send).toHaveBeenLastCalledWith(false, 'view board')
    vi.advanceTimersByTime(BEAT_MS * 3)
    expect(send).toHaveBeenCalledTimes(4)

    stop()
    shown.visibilityState = 'visible'
    shown.dispatchEvent(new Event('visibilitychange'))
    vi.advanceTimersByTime(BEAT_MS * 3)
    expect(send).toHaveBeenCalledTimes(4)
  })

  it('says what the page is on, by kind and count only', () => {
    noteView('board')
    noteCards(231)
    expect(doing()).toBe('view board, 231 cards')
    noteView(null)
    expect(doing()).toBe('view none, 231 cards')
  })

  it('does nothing outside the app', () => {
    const send = vi.fn((_shown: boolean, _on: string) => Promise.resolve())
    beat(page(), send)
    expect(send).not.toHaveBeenCalled()
  })
})
