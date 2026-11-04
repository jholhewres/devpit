import { describe, expect, it } from 'vitest'

import { grownTokens, tokens } from './highlight'
import { ofName } from './languages'

const ts = ofName('ts')
const code = [
  '/* a comment',
  '   over two lines */',
  'const name = "a string" // and a note',
  'function sum(a: number, b = 1e-3): Total {',
  "  return a.value + b + 'x'",
  '}',
  '`a template',
  'over lines`',
  'export type Total = { readonly at: number }',
].join('\n')

describe('a code block highlighted as it is written', () => {
  it('keeps the tokens of the lines it had, the same objects', () => {
    const was = grownTokens(null, 'const a = 1\nconst b = 2\nlet', ts)
    const now = grownTokens(was, `${was.text}\nconst c = 3`, ts)
    expect(now.tokens[0]).toBe(was.tokens[0])
    expect(now.tokens).toEqual(tokens(now.text, ts))
  })

  it('colours, at every length, as the whole block coloured at once', () => {
    let painted = grownTokens(null, '', ts)
    for (let at = 1; at <= code.length; at += 1) {
      painted = grownTokens(painted, code.slice(0, at), ts)
      expect(painted.tokens, `at ${at}`).toEqual(tokens(code.slice(0, at), ts))
    }
  })

  it('colours a block that was edited, not grown, from its start', () => {
    const was = grownTokens(null, 'const a = 1\nconst b = 2', ts)
    expect(grownTokens(was, 'let a = 1\nconst b = 2', ts).tokens).toEqual(tokens('let a = 1\nconst b = 2', ts))
  })
})
