import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { onCameBack, QUIET_MS } from './cameBack'

/*
 * A window restored fires focus and visibilitychange together, and six
 * readers listened to both: one return was two rounds of reads.
 */

const focus = (): void => void window.dispatchEvent(new Event('focus'))
const show = (): void => void document.dispatchEvent(new Event('visibilitychange'))

/* Each test's clock starts well clear of the return an earlier one made. */
let clock = 0

describe('the window coming back', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    clock += QUIET_MS * 10
    vi.setSystemTime(clock)
  })
  afterEach(() => vi.useRealTimers())

  it('is told once to each reader, though focus and visibility both say it', () => {
    const tree = vi.fn()
    const history = vi.fn()
    const dropTree = onCameBack(tree)
    const dropHistory = onCameBack(history)

    show()
    focus()
    expect(tree).toHaveBeenCalledTimes(1)
    expect(history).toHaveBeenCalledTimes(1)
    dropTree()
    dropHistory()
  })

  it('is told again once the quiet has passed', () => {
    const tree = vi.fn()
    const drop = onCameBack(tree)

    focus()
    vi.advanceTimersByTime(QUIET_MS)
    focus()
    expect(tree).toHaveBeenCalledTimes(2)
    drop()
  })

  it('is not a window being hidden', () => {
    const tree = vi.fn()
    const drop = onCameBack(tree)
    Object.defineProperty(document, 'visibilityState', { value: 'hidden', configurable: true })
    try {
      show()
      expect(tree).not.toHaveBeenCalled()
    } finally {
      Reflect.deleteProperty(document, 'visibilityState')
      drop()
    }
  })

  it('is told to nobody who stopped listening', () => {
    const tree = vi.fn()
    onCameBack(tree)()
    focus()
    expect(tree).not.toHaveBeenCalled()
  })
})
