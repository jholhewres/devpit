import { describe, expect, it } from 'vitest'

import { aim, boxOf, moved, shown } from './browsing'

const at = (typed: string): string => {
  const aimed = aim(typed)
  if (!('at' in aimed)) throw new Error(`${typed} was refused: ${aimed.refused}`)
  return aimed.at
}

const refusal = (typed: string): string => {
  const aimed = aim(typed)
  if (!('refused' in aimed)) throw new Error(`${typed} was accepted as ${aimed.at}`)
  return aimed.refused
}

describe('what a person typed', () => {
  /* The one this module exists for. `localhost:3000` parses as the scheme
     `localhost` with the path `3000`, which is a page nobody asked for. */
  it('reads host:port as a host and a port, not as a scheme', () => {
    expect(at('localhost:3000')).toBe('http://localhost:3000/')
    expect(at('127.0.0.1:8080')).toBe('http://127.0.0.1:8080/')
  })

  /* The pane's own placeholder is `localhost:3000`, and it used to open
     `https://`, which essentially no dev server answers. */
  it('assumes http only for this machine, and https for everywhere else', () => {
    expect(at('localhost:3000')).toMatch(/^http:/)
    expect(at('app.localhost')).toMatch(/^http:/)
    expect(at('[::1]:8080')).toMatch(/^http:/)
    expect(at('example.com')).toMatch(/^https:/)
    /* A host that merely ends in the word is not this machine. */
    expect(at('notlocalhost.com')).toMatch(/^https:/)
  })

  it('keeps a scheme somebody typed', () => {
    expect(at('http://localhost:3000')).toBe('http://localhost:3000/')
    expect(at('https://example.com/a/b?c=d')).toBe('https://example.com/a/b?c=d')
  })

  it('guesses https for a bare host, which is the safe half of the guess', () => {
    expect(at('example.com')).toBe('https://example.com/')
    expect(at('sub.example.com/path')).toBe('https://sub.example.com/path')
  })

  /* The same allowlist browser.rs keeps. The window says no before a round
     trip; Rust says no again because a check only the window makes is one a
     caller can skip. */
  it('refuses a scheme the pane cannot show, and names it', () => {
    for (const typed of ['file:///etc/passwd', 'javascript:alert(1)', 'data:text/html,x']) {
      expect(refusal(typed)).toMatch(/http and https/)
    }
  })

  /* devpit does not quietly send what somebody typed to a search engine. */
  it('refuses a sentence rather than searching for it', () => {
    const said = refusal('how do I make coffee')
    expect(said).toMatch(/not an address/)
    expect(said).toMatch(/does not search/)
  })

  it('refuses nothing at all with something to do about it', () => {
    expect(refusal('')).toMatch(/Type an address/)
    expect(refusal('   ')).toMatch(/Type an address/)
  })
})

describe('what the bar shows', () => {
  it('drops the slash a parser adds to a bare host', () => {
    expect(shown('https://example.com/')).toBe('https://example.com')
    expect(shown('http://localhost:3000/')).toBe('http://localhost:3000')
  })

  it('keeps a real path, query and fragment', () => {
    expect(shown('https://example.com/a?b=c')).toBe('https://example.com/a?b=c')
    expect(shown('https://example.com/#top')).toBe('https://example.com/#top')
  })

  it('shows something unparseable as it is rather than as nothing', () => {
    expect(shown('not a url')).toBe('not a url')
  })
})

describe('where the pane sits', () => {
  const rect = (x: number, y: number, w: number, h: number): DOMRect =>
    ({ left: x, top: y, width: w, height: h }) as DOMRect

  it('measures the box in whole pixels', () => {
    expect(boxOf(rect(10.4, 20.6, 800.5, 600.4))).toEqual({
      x: 10,
      y: 21,
      width: 801,
      height: 600,
    })
  })

  /* A native webview is placed against the window and told where to go on
     every change; telling it the same thing again is a round trip per frame. */
  it('only reports a move that is a move', () => {
    const box = { x: 0, y: 0, width: 100, height: 100 }
    expect(moved(null, box)).toBe(true)
    expect(moved(box, { ...box })).toBe(false)
    expect(moved(box, { ...box, x: 1 })).toBe(true)
    expect(moved(box, { ...box, height: 101 })).toBe(true)
  })
})
