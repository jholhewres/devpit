import { describe, expect, it } from 'vitest'

import { paletteColour, plain, rendered } from './blockRender'

const theme = { green: '#00ff00', red: '#ff0000', background: '#000', foreground: '#fff' }

describe('a block drawn back from its output', () => {
  it('resolves redraws and keeps colours as runs, never as markup', async () => {
    const lines = await rendered('\x1b[32mok\x1b[0m <b>\r\nstep 1\rstep 2\r\n\x1b[1;31mfail\x1b[0m\r\n\r\n', 40, theme)
    expect(plain(lines)).toBe('ok <b>\nstep 2\nfail')
    expect(lines[0]![0]).toEqual({ text: 'ok', fg: '#00ff00' })
    expect(lines[2]![0]).toMatchObject({ text: 'fail', fg: '#ff0000', bold: true })
  })

  it('keeps a frame redrawn in place as one frame, not a line per redraw', async () => {
    /* What Ink (Claude Code's TUI) writes for a spinner: clear the last
       frame's lines going up, back to the first column, draw the next. */
    const erase = (lines: number): string => '\x1b[2K' + '\x1b[1A\x1b[2K'.repeat(lines - 1) + '\x1b[G'
    const frame = (glyph: string): string => `${glyph} Pouncing…\r\n\r\nesc to interrupt\r\n`
    const output = 'done earlier\r\n' + frame('✻') + ['✶', '✳', '✢', '·'].map((glyph) => erase(4) + frame(glyph)).join('')
    expect(plain(await rendered(output, 40, theme))).toBe('done earlier\n· Pouncing…\n\nesc to interrupt')
  })

  it('keeps what a clear screen wiped, as a terminal keeps it in its scrollback', async () => {
    /* `clear` without E3, or a watcher redrawing: home, then erase the screen. */
    const output = 'first run\r\nfailed\r\n\x1b[H\x1b[2Jsecond run\r\n'
    expect(plain(await rendered(output, 40, theme))).toBe('first run\nfailed\nsecond run')
  })

  it('wraps at the width it is drawn at', async () => {
    expect((await rendered('x'.repeat(50), 20, theme)).length).toBe(3)
  })

  it('names the 256 colours past the sixteen', () => {
    expect(paletteColour(1, theme)).toBe('#ff0000')
    expect(paletteColour(196, theme)).toBe('#ff0000')
    expect(paletteColour(232, theme)).toBe('#080808')
  })
})
