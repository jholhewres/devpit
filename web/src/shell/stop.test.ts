import { describe, expect, it } from 'vitest'

import { ARMED_MS, isArmed, pressed, same, stops, targetOf } from './stop'

const key = (over: Partial<KeyboardEvent> = {}): KeyboardEvent =>
  ({ key: 'Escape', repeat: false, defaultPrevented: false, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...over }) as KeyboardEvent

describe('escape, twice', () => {
  it('arms on the first press rather than stopping', () => {
    /* One press must not throw away two minutes of work. */
    const press = pressed(null, 'c1:t1', 0)
    expect(press.type).toBe('arm')
  })

  it('stops on a second press inside the window', () => {
    const first = pressed(null, 'c1:t1', 0)
    if (first.type !== 'arm') throw new Error('the first press should arm')
    expect(pressed(first.armed, 'c1:t1', ARMED_MS - 1).type).toBe('stop')
  })

  it('re-arms rather than stopping once the window has passed', () => {
    const first = pressed(null, 'c1:t1', 0)
    if (first.type !== 'arm') throw new Error('the first press should arm')
    expect(pressed(first.armed, 'c1:t1', ARMED_MS + 1).type).toBe('arm')
  })

  it('does not carry an arming across to another turn', () => {
    /* Armed against a turn that has since finished, the press starts over. */
    const first = pressed(null, 'c1:t1', 0)
    if (first.type !== 'arm') throw new Error('the first press should arm')
    expect(pressed(first.armed, 'c1:t2', 1).type).toBe('arm')
  })

  it('names a turn by its conversation and its turn', () => {
    expect(targetOf('c1', 't1')).toBe('c1:t1')
    expect(targetOf('c1', null)).toBe('c1:')
  })

  it('knows an arming that has expired is no arming', () => {
    expect(isArmed({ target: 'c1:t1', until: 10 }, 'c1:t1', 11)).toBe(false)
    expect(isArmed({ target: 'c1:t1', until: 10 }, 'c1:t1', 9)).toBe(true)
  })

  it('compares armings so a lapsed timer clears only its own', () => {
    expect(same({ target: 'a', until: 1 }, { target: 'a', until: 1 })).toBe(true)
    expect(same({ target: 'a', until: 1 }, { target: 'a', until: 2 })).toBe(false)
    expect(same(null, null)).toBe(true)
  })
})

describe('which keystroke counts', () => {
  it('takes a bare Escape', () => {
    expect(stops(key(), false)).toBe(true)
  })

  it('leaves Escape alone while something that owns it is open', () => {
    /* Closing the menu is what the person meant. */
    expect(stops(key(), true)).toBe(false)
  })

  it('ignores a held key, a modifier, and an Escape already handled', () => {
    expect(stops(key({ repeat: true }), false)).toBe(false)
    expect(stops(key({ shiftKey: true }), false)).toBe(false)
    expect(stops(key({ defaultPrevented: true }), false)).toBe(false)
  })

  it('ignores every other key', () => {
    expect(stops(key({ key: 'Enter' }), false)).toBe(false)
  })
})
