import { describe, expect, it } from 'vitest'

import { imagesOf, isCut, textOf } from './cutOutput'

const cut = 'head\n… [cut: 9000 bytes in all]\ntail'

describe('an output the window got cut', () => {
  it('is told by the backend line or a named image, not by a mention of either', () => {
    expect(isCut(cut)).toBe(true)
    expect(isCut(JSON.stringify([{ type: 'text', text: cut }]))).toBe(true)
    expect(isCut(JSON.stringify([{ type: 'image', mime: 'image/png', bytes: 10, cut: true }]))).toBe(true)
    // A file read that holds the marker's source, or JSON with a cut field.
    expect(isCut("const CUT_MARK = '… [cut: '")).toBe(false)
    expect(isCut('{"cut":true,"what":"a setting"}')).toBe(false)
    expect(isCut('ok')).toBe(false)
  })

  it('keeps the text beside the images of a whole output', () => {
    const whole = JSON.stringify([
      { type: 'image', source: { media_type: 'image/png', data: 'AAAA' } },
      { type: 'text', text: 'the page' },
    ])
    expect(textOf(whole)).toBe('the page')
    expect(textOf('plain')).toBe('plain')
  })

  it('shows a whole array that is not content blocks as it is', () => {
    const numbers = JSON.stringify(Array.from({ length: 2000 }, (_, at) => at))
    expect(textOf(numbers)).toBe(numbers)
    expect(textOf('["a","b"]')).toBe('["a","b"]')
    const mixed = JSON.stringify([{ type: 'text', text: 'x' }, 'loose', 7])
    expect(textOf(mixed)).toBe(mixed)
  })

  it('draws only image blocks as images, a document beside them kept as text', () => {
    const whole = JSON.stringify([
      { type: 'document', source: { media_type: 'application/pdf', data: 'JVBERi0x' } },
      { type: 'image', source: { media_type: 'image/png', data: 'AAAA' } },
    ])
    expect(imagesOf(whole)).toEqual(['data:image/png;base64,AAAA'])
    expect(textOf(whole)).toContain('application/pdf')
  })

  it('shows the other blocks of a whole output rather than dropping them', () => {
    const whole = JSON.stringify([
      { type: 'text', text: 'the page' },
      { type: 'resource', resource: { uri: 'file:///a' } },
    ])
    expect(textOf(whole)).toContain('file:///a')
    expect(textOf(whole)).toContain('the page')
  })
})
