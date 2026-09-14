import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import { stylesheet } from './stylesheet'

const SHELL = resolve(process.cwd(), 'src/shell')

/*
 * No class may be told to be in two places at once.
 *
 * `.grip` was the eight edges a frameless window is pulled by, and had been
 * for a long time. A sidebar divider was written with the same name, which put
 * a second bare `.grip { position: absolute; z-index: 3 }` further down the
 * sheet — so every window edge quietly stopped being `fixed` at `z-index:
 * 200`, and the window could only be resized sideways. Nothing failed; it just
 * stopped working.
 *
 * Repeating a bare class is ordinary in this sheet and is not what went wrong:
 * a base rule early and more of it later is how most of these are written.
 * What went wrong is a later rule **contradicting** where an earlier one put
 * the same class, which is never a refinement and always two components
 * sharing a name by accident.
 */

/** The sheet with its comments taken out — prose there names classes too. */
const css = (): string => stylesheet().replace(/\/\*[\s\S]*?\*\//g, '')

/** Where each bare class selector says it is positioned, in order. */
function placements(): Map<string, string[]> {
  const found = new Map<string, string[]>()
  for (const rule of css().split('}')) {
    const [head, body] = rule.split('{')
    if (body === undefined) continue
    const where = /(?:^|;|\s)position:\s*([a-z]+)/.exec(body)?.[1]
    if (!where) continue
    for (const selector of (head ?? '').split(',')) {
      const bare = /^\s*\.([a-z0-9_-]+)\s*$/i.exec(selector)
      if (bare) found.set(bare[1]!, [...(found.get(bare[1]!) ?? []), where])
    }
  }
  return found
}

describe('one class, one place', () => {
  it('never puts the same class somewhere else further down', () => {
    const argued = [...placements()]
      .filter(([, places]) => new Set(places).size > 1)
      .map(([name, places]) => `.${name}: ${[...new Set(places)].join(' then ')}`)
    expect(argued).toEqual([])
  })

  it('would catch the collision that broke the window', () => {
    // The guard only means anything if it can fail.
    const sheet = '.grip { position: fixed; }\n.grip { position: absolute; }'
    const places = [...sheet.matchAll(/\.grip \{[^}]*position:\s*([a-z]+)/g)].map((one) => one[1])
    expect(new Set(places).size).toBe(2)
  })
})

describe('the window edges are still the window edges', () => {
  it('keeps them fixed and above everything', () => {
    // Fixed and a high z-index are what make an edge reachable over the
    // chrome. An absolute one at z-index 3 sits under the top bar.
    expect(css()).toMatch(/\.grip \{[^}]*position: fixed[^}]*z-index: 200/)
  })

  it('has a rule for every edge the window offers', () => {
    const sheet = css()
    const offered = /EDGES[^=]*=\s*\[([^\]]*)\]/.exec(
      readFileSync(resolve(SHELL, 'window.ts'), 'utf8'),
    )?.[1]
    const edges = [...(offered ?? '').matchAll(/'([A-Za-z]+)'/g)].map((one) => one[1]!)
    expect(edges).toHaveLength(8)
    for (const edge of edges) {
      expect(sheet, `${edge} has no rule`).toContain(`.grip[data-edge='${edge}']`)
    }
  })

  it('leaves the divider under its own name', () => {
    expect(css()).toContain(".seam[data-panel='sidebar']")
  })
})

/*
 * Settings sections were touching.
 *
 * The gap was `.pref + .pref`, which only ever separated two switch rows: a
 * card after a switch, a switch after a card, or two cards in a row got
 * nothing, and the pane read as one block with seams in it. Every settings
 * screen had it, because every one of them mixes the two.
 */
describe('a settings pane separates its sections', () => {
  it('gives every child a gap from the one before it', () => {
    expect(css()).toMatch(/\.prefs__in > \* \+ \* \{[^}]*margin-top/)
  })

  it('does not leave the gap depending on what kind of section it is', () => {
    // The rule this replaces. Two rules for one gap disagree eventually.
    expect(css()).not.toContain('.pref + .pref')
  })

  it('still lets a heading open a group', () => {
    const sheet = css()
    expect(sheet).toContain('.prefs__in > .prefs__h + *')
    expect(sheet).toContain('.prefs__in > * + .prefs__h')
  })
})
