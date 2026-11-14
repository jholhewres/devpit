import { afterEach, describe, expect, it } from 'vitest'

import { applyLooks, DEFAULTS, followUpOf, rereadViewPrefs, sendKeyOf, setViewPref, viewPrefs, type ViewPrefs } from './viewPrefs'

afterEach(() => {
  localStorage.clear()
  rereadViewPrefs()
})

/* Each one away from its default, as a person would pick it. */
const PICKED: ViewPrefs = {
  sendKey: 'modEnter',
  followUp: 'steer',
  textSize: 'larger',
  codeSize: 'small',
  contrast: 'more',
  latex: true,
  diffLayout: 'stacked',
  diffWhitespace: 'ignore',
  diffColours: 'safe',
  diffCollapsed: false,
}

describe('the view preferences', () => {
  it('default to what devpit did before they existed', () => {
    expect(viewPrefs()).toEqual({
      sendKey: 'enter',
      followUp: 'queue',
      textSize: 'default',
      codeSize: 'default',
      contrast: 'standard',
      latex: false,
      diffLayout: 'each',
      diffWhitespace: 'show',
      diffColours: 'standard',
      diffCollapsed: true,
    })
  })

  it.each(Object.keys(PICKED) as (keyof ViewPrefs)[])('keep %s across a restart of the window', (key) => {
    setViewPref(key, PICKED[key] as never)
    rereadViewPrefs()
    expect(viewPrefs()[key]).toEqual(PICKED[key])
    expect(viewPrefs()).toEqual({ ...DEFAULTS, [key]: PICKED[key] })
  })

  it('read a value they do not know, or broken storage, as the default', () => {
    localStorage.setItem('devpit.viewPrefs', JSON.stringify({ sendKey: 'space', textSize: 9, latex: 'yes', contrast: 'more' }))
    expect(viewPrefs()).toEqual({ ...DEFAULTS, contrast: 'more' })
    rereadViewPrefs()
    localStorage.setItem('devpit.viewPrefs', '{not json')
    expect(viewPrefs()).toEqual(DEFAULTS)
  })
})

describe('the send key', () => {
  it('by default: Enter sends, Shift+Enter is a new line, Ctrl+Enter sends the other way', () => {
    expect(sendKeyOf({ key: 'Enter' }, 'enter')).toBe('send')
    expect(sendKeyOf({ key: 'Enter', shiftKey: true }, 'enter')).toBeNull()
    expect(sendKeyOf({ key: 'Enter', ctrlKey: true }, 'enter')).toBe('other')
    expect(sendKeyOf({ key: 'a' }, 'enter')).toBeNull()
  })

  it('set to Ctrl+Enter: Enter is a new line and Ctrl+Enter sends', () => {
    expect(sendKeyOf({ key: 'Enter' }, 'modEnter')).toBeNull()
    expect(sendKeyOf({ key: 'Enter', shiftKey: true }, 'modEnter')).toBeNull()
    expect(sendKeyOf({ key: 'Enter', ctrlKey: true }, 'modEnter')).toBe('send')
  })

  it('queues or steers a message sent during a turn, and the other key does the opposite', () => {
    expect(followUpOf({ followUp: 'queue' }, false)).toBe('queue')
    expect(followUpOf({ followUp: 'queue' }, true)).toBe('steer')
    expect(followUpOf({ followUp: 'steer' }, false)).toBe('steer')
    expect(followUpOf({ followUp: 'steer' }, true)).toBe('queue')
  })
})

describe('the looks', () => {
  it('leave the stylesheet at its own sizes and colours by default', () => {
    const root = document.createElement('html')
    applyLooks(DEFAULTS, root)
    expect(root.style.getPropertyValue('--text-scale')).toBe('1')
    expect(root.style.getPropertyValue('--code-scale')).toBe('1')
    expect(root.dataset.contrast).toBe('standard')
    expect(root.dataset.diffColours).toBe('standard')
  })

  it('scale the text and the code apart, and set the contrast and diff colours', () => {
    const root = document.createElement('html')
    applyLooks(PICKED, root)
    expect(root.style.getPropertyValue('--text-scale')).toBe('1.2')
    expect(root.style.getPropertyValue('--code-scale')).toBe('0.92')
    expect(root.dataset.contrast).toBe('more')
    expect(root.dataset.diffColours).toBe('safe')
  })
})
