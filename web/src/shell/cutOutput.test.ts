import { describe, expect, it } from 'vitest'

import { isCut, textOf } from './cutOutput'

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
})
