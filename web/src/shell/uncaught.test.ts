import { afterEach, describe as group, expect, it, vi } from 'vitest'

import { MOST_PER_MINUTE, QUIET_MS, describe, reportUncaught, sparing } from './uncaught'

afterEach(() => {
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
})

group('uncaught errors', () => {
  it('hands a thrown error and a rejection to the report', () => {
    ;(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {}
    const target = new EventTarget() as unknown as Window
    const send = vi.fn(() => Promise.resolve({ status: 'ok' as const, data: null }))
    reportUncaught(target, send)

    const error = new TypeError('x is undefined')
    target.dispatchEvent(Object.assign(new Event('error'), { error }))
    target.dispatchEvent(Object.assign(new Event('unhandledrejection'), { reason: 'gone' }))

    expect(send).toHaveBeenNthCalledWith(1, 'TypeError: x is undefined', error.stack)
    expect(send).toHaveBeenNthCalledWith(2, 'gone', null)
  })

  it('listens to nothing outside the app', () => {
    const target = new EventTarget() as unknown as Window
    const send = vi.fn()
    reportUncaught(target, send)
    target.dispatchEvent(Object.assign(new Event('error'), { error: new Error('x') }))
    expect(send).not.toHaveBeenCalled()
  })

  it('says what a non-error was', () => {
    expect(describe(42)).toEqual({ message: '42', stack: null })
  })
})

group('a storm of errors', () => {
  it('sends the same message once per quiet spell', () => {
    let clock = 0
    const allowed = sparing(() => clock)
    expect(allowed('boom')).toBe(true)
    expect(allowed('boom')).toBe(false)
    clock = QUIET_MS
    expect(allowed('boom')).toBe(true)
  })

  it('sends no more than the ceiling in a minute, whatever the messages', () => {
    const allowed = sparing(() => 0)
    const sent = Array.from({ length: MOST_PER_MINUTE + 5 }, (_, n) => allowed(`error ${n}`)).filter(Boolean)
    expect(sent).toHaveLength(MOST_PER_MINUTE)
  })
})
