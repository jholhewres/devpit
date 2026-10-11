import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import { stylesheet } from './stylesheet'

/*
 * The hand comes from one rule in `index.css`. Parts that each declared it
 * again are how it drifted: a hand on some buttons, an arrow on the rest.
 */

const withoutComments = (css: string): string => css.replace(/\/\*[\s\S]*?\*\//g, '')

/** The selectors of every rule in `css` that sets this cursor. */
function setting(css: string, cursor: string): string[] {
  return withoutComments(css)
    .split('}')
    .filter((rule) => new RegExp(`cursor:\\s*${cursor}\\b`).test(rule))
    .map((rule) => rule.split('{')[0]!.trim().replace(/\s+/g, ' '))
}

describe('the cursor', () => {
  it('is the hand on controls because of the base rule, not because each part says so', () => {
    // Neither is a control the base can see: a header that toggles its block,
    // and a widget CodeMirror draws.
    expect(setting(stylesheet(), 'pointer')).toEqual(['.mdiff .cm-collapsedLines', '.blk__head'])
  })

  it('is the arrow on a disabled control, from the same base rule', () => {
    expect(setting(stylesheet(), 'default').filter((selector) => selector.includes(':disabled'))).toEqual([])
    const base = withoutComments(readFileSync(resolve(process.cwd(), 'src/index.css'), 'utf8'))
    expect(setting(base, 'default')).toEqual([":where(:disabled, [aria-disabled='true'])"])
  })

  it('would catch a part that gives a button its own hand again', () => {
    expect(setting('.x { color: red; }\n.y { cursor: pointer; }', 'pointer')).toEqual(['.y'])
  })
})
