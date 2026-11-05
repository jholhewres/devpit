import { afterEach, describe, expect, it, vi } from 'vitest'

import { BEAT_MS, beat, doing, noteCards, noteView } from './beat'

afterEach(() => {
  vi.useRealTimers()
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
  delete (window as unknown as { __DEVPIT_BROWSER__?: unknown }).__DEVPIT_BROWSER__
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

  /* The page in a browser reaches the app's calls too, and its beat would
     stand for the window's in the app's stall report. */
  it('does nothing in a browser tab, which is not the window', () => {
    Object.assign(window, { __TAURI_INTERNALS__: {}, __DEVPIT_BROWSER__: true })
    const send = vi.fn((_shown: boolean, _on: string) => Promise.resolve())
    beat(page(), send)
    expect(send).not.toHaveBeenCalled()
  })
})
