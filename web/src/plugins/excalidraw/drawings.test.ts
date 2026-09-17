import { describe, expect, it } from 'vitest'

import { drawingFileValid } from './drawingFile'
import { fileName } from '../pluginFile'
import { DRAWING, drawingTab, plainDataName, stemOf } from './drawings'

const drawingName = (typed: string) => fileName(DRAWING, typed)

describe('drawingName', () => {
  it('adds the extension to a typed name', () => {
    expect(drawingName('fluxo')).toEqual({ name: 'fluxo.excalidraw' })
    expect(drawingName('  fluxo  ')).toEqual({ name: 'fluxo.excalidraw' })
  })

  it('does not add the extension twice when it was typed', () => {
    expect(drawingName('fluxo.excalidraw')).toEqual({ name: 'fluxo.excalidraw' })
  })

  it('accepts what the backend accepts, spaces and accents included', () => {
    expect(drawingName('Projeto São João')).toEqual({ name: 'Projeto São João.excalidraw' })
    expect(drawingName('v2_draft-1')).toEqual({ name: 'v2_draft-1.excalidraw' })
  })

  it('refuses an empty name', () => {
    expect(drawingName('')).toHaveProperty('problem')
    expect(drawingName('   ')).toHaveProperty('problem')
    expect(drawingName('.excalidraw')).toHaveProperty('problem')
  })

  it('refuses what the backend refuses: a leading dot, "..", separators, NUL and ":"', () => {
    for (const typed of ['.hidden', 'a..b', 'a/b', 'a\\b', 'a\0b', 'c:d']) {
      expect(drawingName(typed), typed).toHaveProperty('problem')
    }
  })
})

describe('plainDataName', () => {
  it('mirrors home::plain_data_name', () => {
    expect(plainDataName('fluxo.excalidraw')).toBe(true)
    expect(plainDataName('')).toBe(false)
    expect(plainDataName('.fluxo.excalidraw')).toBe(false)
    expect(plainDataName('a..excalidraw')).toBe(false)
    expect(plainDataName('/abs.excalidraw')).toBe(false)
    expect(plainDataName('C:\\x.excalidraw')).toBe(false)
  })

  // Same cases as home_tests.rs's a_data_name_windows_or_a_listing_would_misread_is_refused.
  it('refuses a name Windows or a listing would misread', () => {
    // 190 + 11 is one byte past the 200 a name may have.
    const long = `${'a'.repeat(190)}.excalidraw`
    for (const name of [
      'con.excalidraw',
      'NUL.excalidraw',
      'Com1.excalidraw',
      'lpt9.old.excalidraw',
      'aux .excalidraw',
      'flow.excalidraw.',
      'flow.excalidraw ',
      'a<b.excalidraw',
      'a>b.excalidraw',
      'a"b.excalidraw',
      'a|b.excalidraw',
      'a?b.excalidraw',
      'a*b.excalidraw',
      'a\u{7}b.excalidraw',
      'a\u{202E}b.excalidraw',
      'a\u{2067}b.excalidraw',
      long,
    ]) {
      expect(plainDataName(name), name).toBe(false)
    }
  })

  // Same cases as home_tests.rs's a_data_name_that_only_starts_like_a_device_is_kept.
  it('accepts a name that only starts like a device', () => {
    // 189 + 11 is exactly the 200 a name may have.
    const longest = `${'a'.repeat(189)}.excalidraw`
    for (const name of [
      'console.excalidraw',
      'com10.excalidraw',
      'nullish.excalidraw',
      'Überblick.excalidraw',
      'flow (2).excalidraw',
      longest,
    ]) {
      expect(plainDataName(name), name).toBe(true)
    }
  })

  it('counts UTF-8 bytes for the length ceiling, not UTF-16 units', () => {
    // 'é' is one JS string unit but two UTF-8 bytes, so the string length
    // (106) stays well under 200 while the byte length (201) goes past it.
    const overByBytesOnly = `${'é'.repeat(95)}.excalidraw`
    expect(overByBytesOnly.length).toBeLessThan(200)
    expect(new TextEncoder().encode(overByBytesOnly).length).toBe(201)
    expect(plainDataName(overByBytesOnly)).toBe(false)

    // One 'é' fewer lands exactly on the 200-byte ceiling.
    const atTheByteCeiling = `${'é'.repeat(94)}.excalidraw`
    expect(new TextEncoder().encode(atTheByteCeiling).length).toBe(199)
    expect(plainDataName(atTheByteCeiling)).toBe(true)
  })
})

describe('drawingTab', () => {
  it('names a tab after its file, so the same file is the same tab', () => {
    expect(drawingTab('fluxo.excalidraw')).toEqual({
      id: 'drawing:fluxo.excalidraw',
      path: 'fluxo.excalidraw',
      title: 'fluxo',
    })
    expect(drawingTab('fluxo.excalidraw').id).toBe(drawingTab('fluxo.excalidraw').id)
  })

  it('titles the tab without the extension', () => {
    expect(stemOf('mapa.excalidraw')).toBe('mapa')
  })
})

describe('DRAWING.empty', () => {
  it('is a file the plugin itself would open', () => {
    expect(drawingFileValid(DRAWING.empty)).toBe(true)
  })
})
