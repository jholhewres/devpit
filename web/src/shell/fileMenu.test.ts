import { describe, expect, it } from 'vitest'

import { FILE_MENU, wired, type Entry } from './fileMenu'

describe('the file context menu', () => {
  it('has something behind every entry that is not a rule', () => {
    expect(wired(FILE_MENU)).toBe(true)
  })

  /* The break-it-on-purpose case, kept rather than performed once: a label
     with no action is exactly the defect this menu used to be made of, and
     without this the first test would keep passing after one crept back. */
  it('fails on an entry that only looks like a control', () => {
    const dead: readonly Entry[] = [...FILE_MENU, { label: 'Does nothing' }]
    expect(wired(dead)).toBe(false)
  })
})
