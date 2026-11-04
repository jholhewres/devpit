import { describe, expect, it } from 'vitest'

import { blocks, grown, mended } from './markdown'

const answer = [
  '# Plan',
  'First a paragraph that runs',
  'over two lines.',
  '',
  '- one `code`',
  '- two **bold**',
  '',
  '| a | b |',
  '|:--|--:|',
  '| 1 | 2 |',
  '',
  'Prose right above a table',
  '| x | y |',
  '|---|---|',
  '| 3 | 4 |',
  '',
  '> quoted',
  '> twice',
  '',
  '```ts',
  'const a = 1',
  '',
  'const b = 2',
  '```',
  '',
  '---',
  '',
  'The end.',
].join('\n')

describe('an answer parsed as it arrives', () => {
  it('keeps the blocks it had closed, the same objects', () => {
    const was = grown(null, 'para one\n\npara two\n\npar')
    const now = grown(was, `${was.source}a three\n\n- item`)
    expect(now.blocks[0]).toBe(was.blocks[0])
    expect(now.blocks[1]).toBe(was.blocks[1])
    expect(now.blocks).toEqual(blocks(now.source))
  })

  it('reads, at every length, as the whole text read at once', () => {
    let parsed = grown(null, '')
    for (let at = 1; at <= answer.length; at += 1) {
      parsed = grown(parsed, answer.slice(0, at))
      expect(parsed.blocks, `at ${at}`).toEqual(blocks(answer.slice(0, at)))
    }
  })

  it('reads in uneven pieces as the whole text read at once', () => {
    let parsed = grown(null, '')
    for (let at = 0; at < answer.length; ) {
      at = Math.min(answer.length, at + 1 + ((at * 7) % 23))
      parsed = grown(parsed, answer.slice(0, at))
      expect(parsed.blocks, `at ${at}`).toEqual(blocks(answer.slice(0, at)))
    }
  })

  it('reads a text that was replaced, not grown, from its start', () => {
    const was = grown(null, 'one\n\ntwo\n\nthree')
    const now = grown(was, 'uno\n\ntwo\n\nthree')
    expect(now.blocks).toEqual(blocks('uno\n\ntwo\n\nthree'))
    // An edit above leaves the blocks that read the same as they were.
    expect(now.blocks[1]).toBe(was.blocks[1])
  })
})

describe('the block still being written', () => {
  it('closes the bold and the code it left open', () => {
    expect(mended({ kind: 'paragraph', text: 'so **bold and `code' })).toEqual({ kind: 'paragraph', text: 'so **bold and `code`**' })
  })

  it('leaves a block with nothing open as it was', () => {
    const block = { kind: 'paragraph', text: 'so **bold** and `code`' } as const
    expect(mended(block)).toBe(block)
  })
})
