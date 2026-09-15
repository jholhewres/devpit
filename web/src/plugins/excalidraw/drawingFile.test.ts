import { describe, expect, it } from 'vitest'

import { drawingFileValid, pasteAllowed, unsafeElement, withoutFrames } from './drawingFile'

const valid = JSON.stringify({ type: 'excalidraw', version: 2, elements: [] })

/* The shape Excalidraw 0.18 turns into a live `srcdoc`. */
const IFRAME = {
  id: 'f',
  type: 'iframe',
  customData: { generationData: { status: 'done', html: '<script>alert(1)</script>' } },
}
const EMBEDDABLE = { id: 'e', type: 'embeddable', link: 'https://example.com' }
const RECTANGLE = { id: 'r', type: 'rectangle' }
const drawingWith = (...elements: unknown[]): string => JSON.stringify({ type: 'excalidraw', version: 2, elements })

describe('drawingFileValid', () => {
  it('accepts what serializeAsJSON writes', () => {
    expect(drawingFileValid(valid)).toBe(true)
    expect(drawingFileValid(JSON.stringify({ type: 'excalidraw', version: 2, elements: [{ id: '1' }] }))).toBe(true)
  })

  it('rejects text that is not JSON', () => {
    expect(drawingFileValid('not json')).toBe(false)
    expect(drawingFileValid('')).toBe(false)
  })

  it('rejects a JSON value that is not an object', () => {
    expect(drawingFileValid('42')).toBe(false)
    expect(drawingFileValid('null')).toBe(false)
    expect(drawingFileValid('[1,2,3]')).toBe(false)
  })

  it('rejects anything that is not the excalidraw type', () => {
    expect(drawingFileValid(JSON.stringify({ type: 'other', version: 2, elements: [] }))).toBe(false)
    expect(drawingFileValid(JSON.stringify({ version: 2, elements: [] }))).toBe(false)
  })

  it('rejects a version other than 2', () => {
    expect(drawingFileValid(JSON.stringify({ type: 'excalidraw', version: 1, elements: [] }))).toBe(false)
    expect(drawingFileValid(JSON.stringify({ type: 'excalidraw', version: '2', elements: [] }))).toBe(false)
    expect(drawingFileValid(JSON.stringify({ type: 'excalidraw', elements: [] }))).toBe(false)
  })

  it('rejects a missing or non-array elements field', () => {
    expect(drawingFileValid(JSON.stringify({ type: 'excalidraw', version: 2 }))).toBe(false)
    expect(drawingFileValid(JSON.stringify({ type: 'excalidraw', version: 2, elements: {} }))).toBe(false)
    expect(drawingFileValid(JSON.stringify({ type: 'excalidraw', version: 2, elements: 'x' }))).toBe(false)
  })

  it('rejects a drawing with an iframe element that carries its own HTML', () => {
    expect(drawingFileValid(drawingWith(RECTANGLE, IFRAME))).toBe(false)
  })

  it('rejects a drawing with an embeddable element', () => {
    expect(drawingFileValid(drawingWith(EMBEDDABLE, RECTANGLE))).toBe(false)
  })
})

describe('unsafeElement', () => {
  it('is an iframe or an embeddable, and nothing else', () => {
    expect(unsafeElement(IFRAME)).toBe(true)
    expect(unsafeElement(EMBEDDABLE)).toBe(true)
    for (const other of [RECTANGLE, { type: 'image' }, { type: 'frame' }, {}, null, 42]) {
      expect(unsafeElement(other), JSON.stringify(other)).toBe(false)
    }
  })
})

describe('withoutFrames', () => {
  it('drops iframes and embeddables and keeps every other element in order', () => {
    const text = { id: 't', type: 'text' }
    expect(withoutFrames([IFRAME, RECTANGLE, EMBEDDABLE, text])).toEqual([RECTANGLE, text])
  })
})

describe('pasteAllowed', () => {
  it('refuses pasted elements that include an iframe or an embeddable', () => {
    expect(pasteAllowed({ elements: [RECTANGLE, IFRAME] })).toBe(false)
    expect(pasteAllowed({ elements: [EMBEDDABLE] })).toBe(false)
  })

  it('lets through other elements, and a paste with no elements at all', () => {
    expect(pasteAllowed({ elements: [RECTANGLE] })).toBe(true)
    expect(pasteAllowed({})).toBe(true)
  })
})
