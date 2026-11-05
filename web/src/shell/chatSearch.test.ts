import { describe, expect, it } from 'vitest'

import type { Message } from '../gen/bindings'
import { counter, occurrences, pagesUntil, rangesIn, stepped } from './chatSearch'

const said = (id: string): Message => ({ id, turnId: null, role: 'assistant', parts: [{ kind: 'text', text: id, parent: null }], createdAt: 0, streaming: false, ended: null }) as Message

/* A transcript of `size` messages, read a page at a time from its end, as chat.history reads it. */
function transcript(size: number, most: number) {
  const all = Array.from({ length: size }, (_, at) => said(`m${at}`))
  let reads = 0
  const read = async (before: string) => {
    reads += 1
    const end = all.findIndex((one) => one.id === before)
    const start = Math.max(0, end - most)
    return { messages: all.slice(start, end), older: start > 0 }
  }
  return { all, read, reads: () => reads }
}

describe('a search over the whole conversation', () => {
  it('counts every match in every message, in order', () => {
    const found = occurrences([
      { messageId: 'm3', count: 2 },
      { messageId: 'm9', count: 1 },
    ])
    expect(found).toEqual([
      { messageId: 'm3', nth: 0 },
      { messageId: 'm3', nth: 1 },
      { messageId: 'm9', nth: 0 },
    ])
    expect(counter(1, found.length)).toBe('2 of 3')
    expect(counter(0, 0)).toBe('No results')
  })

  it('goes round the ends', () => {
    expect(stepped(2, 3, false)).toBe(0)
    expect(stepped(0, 3, true)).toBe(2)
  })

  it('reads the pages above until the match is among them, in a conversation of 2,000 messages', async () => {
    const { all, read, reads } = transcript(2000, 500)
    const got = await pagesUntil('m120', 'm1950', read)
    expect(got.found).toBe(true)
    expect(got.messages[0]!.id).toBe('m0')
    expect(got.messages.at(-1)!.id).toBe('m1949')
    expect(got.messages.length).toBe(1950)
    expect(got.older).toBe(false)
    expect(reads()).toBe(4)
    expect(all.length).toBe(2000)
  })

  it('stops at the top when the match is not there', async () => {
    const { read } = transcript(30, 10)
    const got = await pagesUntil('elsewhere', 'm20', read)
    expect(got).toMatchObject({ found: false, older: false })
    expect(got.messages.length).toBe(20)
  })
})

describe('the word where it is drawn', () => {
  it('is found in prose only, ignoring case', () => {
    const root = document.createElement('div')
    root.innerHTML = '<div class="md"><p>Parser and <b>parser</b></p></div><div class="acts"><div class="md">parser</div></div><div class="said__t">PARSER</div>'
    const ranges = rangesIn(root, 'parser')
    expect(ranges.map((one) => one.toString())).toEqual(['Parser', 'parser', 'PARSER'])
  })
})
