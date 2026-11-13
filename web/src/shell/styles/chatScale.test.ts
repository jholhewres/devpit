import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

/* Read off disk: `?raw` comes back empty under vitest. */
const read = (path: string): string => readFileSync(resolve(process.cwd(), path), 'utf8')

const CHAT = ['src/shell/styles/14-chat.css', 'src/shell/styles/21-turn.css']
const DECLARATION = /(?<![\w-])(font-size|font|border(?:-[a-z]+)*-radius)\s*:\s*([^;}]+)/g

/** Every font size and radius in `css` written as a length instead of a step of the scale. */
function offScale(css: string): string[] {
  return css.split('\n').flatMap((line, at) =>
    [...line.matchAll(DECLARATION)]
      .filter((m) => /\d(px|r?em|pt)\b/.test(m[2]))
      .map((m) => `line ${at + 1}: ${m[1]}: ${m[2].trim()}`),
  )
}

describe("the chat's type and radii", () => {
  it('catches a size or a radius written as a length', () => {
    const literal = '.a { font-size: 12px; border-radius: 6px; font: 600 12.5px var(--mono); border-top-left-radius: 2px; }'
    expect(offScale(literal)).toHaveLength(4)
    expect(offScale('.a { font-size: var(--fs-sm); border-radius: 50%; font: inherit; }')).toEqual([])
  })

  it.each(CHAT)('%s takes every font size and radius from the scale', (path) => {
    expect(offScale(read(path))).toEqual([])
  })

  it.each(CHAT)('%s names only steps tokens.css defines', (path) => {
    const tokens = read('src/theme/tokens.css')
    const used = new Set([...read(path).matchAll(/var\((--(?:fs|radius)-[a-z0-9]+)\)/g)].map((m) => m[1]))
    expect([...used].filter((name) => !tokens.includes(`${name}:`))).toEqual([])
  })
})
