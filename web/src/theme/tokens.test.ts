import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

/*
 * The stylesheet is read off disk rather than imported: `?raw` comes back
 * empty under vitest, and a test that asserts about an empty string passes
 * for the wrong reason.
 */
const css = readFileSync(resolve(process.cwd(), 'src/theme/tokens.css'), 'utf8')

/** The declarations inside the first block whose selector matches. */
function tokensIn(selector: string): Set<string> {
  const at = css.indexOf(selector)
  expect(at, `${selector} is not in tokens.css`).toBeGreaterThan(-1)
  const open = css.indexOf('{', at)
  let depth = 0
  let close = open
  for (; close < css.length; close += 1) {
    if (css[close] === '{') depth += 1
    if (css[close] === '}') {
      depth -= 1
      if (depth === 0) break
    }
  }
  return new Set(
    [...css.slice(open, close).matchAll(/(--[a-z0-9-]+)\s*:/g)].map((m) => m[1]),
  )
}

/* Sizes and faces are the same in every theme; only colour changes. */
const NOT_A_COLOUR = new Set(['--sidebar-w', '--content-max', '--files-w', '--ui', '--mono'])

describe('the palette', () => {
  const dark = tokensIn(':root {')

  it('has every dark token in the light theme too', () => {
    const light = tokensIn(":root[data-theme='light']")
    const missing = [...dark].filter((token) => !NOT_A_COLOUR.has(token) && !light.has(token))
    expect(missing, 'a token with no light value paints one theme on the other').toEqual([])
  })

  it('has every dark token in the system theme too', () => {
    const system = tokensIn(":root[data-theme='system']")
    const missing = [...dark].filter((token) => !NOT_A_COLOUR.has(token) && !system.has(token))
    expect(missing).toEqual([])
  })

  it('names more than a handful of tokens', () => {
    /* Guards the reader above: an empty file would satisfy the parity checks. */
    expect(dark.size).toBeGreaterThan(30)
  })

  it('keeps the terminal dark in the light theme', () => {
    /* A shell paints its own ANSI palette for a dark ground. Repainting the
       terminal white leaves those colours on the wrong background. */
    const light = css.slice(css.indexOf(":root[data-theme='light']"))
    expect(light).toMatch(/--terminal:\s*#151515/)
  })
})
