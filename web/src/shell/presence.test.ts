import { afterEach, describe, expect, it, vi } from 'vitest'

import { EVERY_MS, reportPresence } from './presence'

afterEach(() => {
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
})

describe('presence', () => {
  it('says the window is in use at most once a minute', () => {
    ;(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {}
    let clock = 0
    const target = new EventTarget() as unknown as Window
    const seen = vi.fn(() => Promise.resolve({ status: 'ok' as const, data: null }))
    reportPresence(target, seen, () => clock)

    target.dispatchEvent(new Event('keydown'))
    target.dispatchEvent(new Event('pointerdown'))
    expect(seen).toHaveBeenCalledTimes(1)
    clock = EVERY_MS
    target.dispatchEvent(new Event('wheel'))
    expect(seen).toHaveBeenCalledTimes(2)
  })
})
