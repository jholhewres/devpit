import { describe, expect, it } from 'vitest'

import { advanced, cut, locate, opened, styleOf } from './veil'

describe('what has just arrived', () => {
  it('leaves the text a message opened with alone', () => {
    /* Re-opening a finished conversation should not replay it. */
    const veil = opened('already here')
    expect(advanced(veil, 'already here', true, 0)).toEqual([])
  })

  it('marks only the appended tail', () => {
    const veil = opened('one')
    const chunks = advanced(veil, 'one two', true, 0)
    expect(chunks).toHaveLength(1)
    expect(chunks[0]).toMatchObject({ start: 3, end: 7 })
  })

  it('keeps earlier deltas fading on their own clock', () => {
    /* Two chunks in flight at once, each started when it arrived. */
    const veil = opened('')
    advanced(veil, 'a', true, 0)
    const chunks = advanced(veil, 'ab', true, 100)
    expect(chunks).toHaveLength(2)
    expect(chunks[0]!.at).toBe(0)
    expect(chunks[1]!.at).toBe(100)
  })

  it('retires a chunk once its fade is over', () => {
    const veil = opened('')
    advanced(veil, 'a', true, 0)
    expect(advanced(veil, 'a', true, 5000)).toEqual([])
  })

  it('trims a chunk the provider overwrote instead of dropping it whole', () => {
    const veil = opened('')
    advanced(veil, 'hello world', true, 0)
    const chunks = advanced(veil, 'hello there', true, 10)
    /* `hello ` survived; what came after it was replaced. */
    expect(chunks.some((chunk) => chunk.start === 0 && chunk.end === 6)).toBe(true)
  })

  it('forgets everything the moment the turn settles', () => {
    const veil = opened('')
    advanced(veil, 'a', true, 0)
    expect(advanced(veil, 'a', false, 1)).toEqual([])
  })
})

describe('how long the fade lasts', () => {
  it('is bounded however slow the stream is', () => {
    const veil = opened('')
    let text = ''
    for (let at = 0; at < 6; at += 1) {
      text += 'x'
      advanced(veil, text, true, at * 5000)
    }
    for (const chunk of veil.chunks) expect(chunk.ms).toBeLessThanOrEqual(400)
  })

  it('is bounded however fast the stream is', () => {
    const veil = opened('')
    let text = ''
    for (let at = 0; at < 6; at += 1) {
      text += 'x'
      advanced(veil, text, true, at)
    }
    for (const chunk of veil.chunks) expect(chunk.ms).toBeGreaterThanOrEqual(120)
  })

  it('resumes rather than restarts a chunk already in flight', () => {
    /* Without the negative delay every React render would replay the fade. */
    const style = styleOf({ start: 0, end: 1, at: 0, ms: 300 }, 100, 1)
    expect(style['--veil-delay' as keyof typeof style]).toBe('-100ms')
  })
})

describe('cutting a rendered run', () => {
  const chunks = [{ start: 4, end: 9, at: 0, ms: 300 }]

  it('leaves a run no chunk touches in one piece', () => {
    expect(cut('abcd', 0, chunks)).toEqual([{ text: 'abcd', chunk: null }])
  })

  it('splits a run the chunk starts inside', () => {
    const pieces = cut('abcdefgh', 0, chunks)
    expect(pieces.map((piece) => piece.text)).toEqual(['abcd', 'efgh'])
    expect(pieces[0]!.chunk).toBeNull()
    expect(pieces[1]!.chunk).not.toBeNull()
  })

  it('does nothing when nothing is fading', () => {
    expect(cut('abcd', 0, [])).toEqual([{ text: 'abcd', chunk: null }])
  })
})

describe('finding a run in the source', () => {
  it('walks forward so a repeated word lands on its own occurrence', () => {
    const source = 'the cat and the hat'
    const first = locate(source, 'the', 0)
    expect(first).toBe(0)
    expect(locate(source, 'the', first + 3)).toBe(12)
  })

  it('reports nothing rather than guessing when the run is not there', () => {
    /* Which only costs that run its fade. */
    expect(locate('abc', 'zzz', 0)).toBe(-1)
  })
})
