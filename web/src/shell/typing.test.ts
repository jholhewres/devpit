import { describe, expect, it } from 'vitest'

import { abandoned, committed, composing } from './typing'

/*
 * The bug: typing `á` as `'` then `a` sends an `Enter` to commit the composed
 * character, and a field that reads that as "done" submits halfway through a
 * word — losing the accent and closing the form on a value nobody finished.
 *
 * Found by somebody typing an accented letter into a card's title.
 */

const key = (over: Record<string, unknown> = {}) => ({ key: 'Enter', ...over }) as never

describe('a keystroke that is still composing a character', () => {
  it('is recognised by the modern signal', () => {
    expect(composing({ isComposing: true })).toBe(true)
  })

  /* React wraps the event; the flag is on the native one underneath. */
  it('is recognised through React’s wrapper', () => {
    expect(composing({ nativeEvent: { isComposing: true } })).toBe(true)
  })

  /* The older spelling of the same fact. WebKit still emits it on some
     paths, which is why checking one signal is how this gets fixed twice. */
  it('is recognised by the legacy keyCode', () => {
    expect(composing({ keyCode: 229 })).toBe(true)
  })

  it('is not recognised where there is nothing to recognise', () => {
    expect(composing({})).toBe(false)
    expect(composing({ isComposing: false, keyCode: 13 })).toBe(false)
  })
})

describe('Enter', () => {
  it('means it when nothing is being composed', () => {
    expect(committed(key())).toBe(true)
  })

  it('does not mean it while a character is being composed', () => {
    expect(committed(key({ isComposing: true }))).toBe(false)
    expect(committed(key({ nativeEvent: { isComposing: true } }))).toBe(false)
    expect(committed(key({ keyCode: 229 }))).toBe(false)
  })

  it('is not any other key', () => {
    expect(committed(key({ key: 'a' }))).toBe(false)
    expect(committed(key({ key: 'Escape' }))).toBe(false)
  })
})

describe('Escape', () => {
  it('means it when nothing is being composed', () => {
    expect(abandoned(key({ key: 'Escape' }))).toBe(true)
  })

  /* An IME uses Escape to abandon what is being composed. A dialog that
     closes on that takes the whole form with the half-typed word. */
  it('is the IME abandoning a character, not the person abandoning the form', () => {
    expect(abandoned(key({ key: 'Escape', isComposing: true }))).toBe(false)
  })
})
