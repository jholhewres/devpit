import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

/*
 * Where the terminal's padding is allowed to live.
 *
 * This is a CSS rule with a behavioural consequence, which is exactly the kind
 * nobody can see in a diff. `FitAddon.proposeDimensions` sizes the grid as:
 *
 *   rows = floor((computed height of the PARENT - padding read off .xterm)
 *                / cell height)
 *
 * It reads the height from the parent and the padding from `.xterm`. Padding
 * put on the parent is therefore height the addon never subtracts, and the
 * grid comes out one row too tall.
 *
 * Measured in a browser with the real addon, an 800x600 pane, 13px/1.2 type:
 * padding on the host asked for 33 rows where 32 fit and drew the 33rd two
 * pixels past the pane, under an `overflow: hidden`. Moving the same padding
 * onto `.xterm` gave 32 rows and 16px to spare.
 *
 * That one clipped row is the bottom line cut in half in a terminal with an
 * agent open — the status line it writes last.
 */

/* From the project root rather than from `import.meta.url`: these run under
   jsdom, where that is an http URL and `readFileSync` refuses it. */
const css = readFileSync(resolve(process.cwd(), 'src/shell/shell.css'), 'utf8')

const ruleFor = (selector: string): string => {
  const at = css.indexOf(`\n${selector} {`)
  expect(at, `no rule for \`${selector}\``).toBeGreaterThan(-1)
  return css.slice(at, css.indexOf('}', at))
}

describe('the terminal grid fits the box it is drawn in', () => {
  it('keeps the padding off the element FitAddon measures the height of', () => {
    expect(ruleFor('.termhost')).not.toMatch(/padding/)
  })

  it('puts it on the element FitAddon reads the padding off', () => {
    expect(ruleFor('.termhost .xterm')).toMatch(/padding:/)
  })

  /* `height: 100%` is what makes the two agree: the grid is measured against
     the parent, so the element has to be the parent's size. */
  it('gives the terminal the full height of its host', () => {
    expect(ruleFor('.termhost .xterm')).toMatch(/height:\s*100%/)
  })

  /* Padding on `.xterm` has a second consequence, and it is visible rather
     than geometric: xterm.css paints `.xterm-viewport` `#000` and positions it
     absolutely against the *padding* box, so the padding shows up as a black
     frame around the terminal in xterm's colour rather than ours. Measured:
     viewport `rgb(0, 0, 0)` over a pane of `rgb(21, 21, 21)`. */
  it('does not let xterm paint its own black over that padding', () => {
    expect(ruleFor('.termhost .xterm .xterm-viewport')).toMatch(
      /background-color:\s*transparent/,
    )
  })
})
