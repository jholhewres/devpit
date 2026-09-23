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

  it('wraps at the width it is drawn at', async () => {
    expect((await rendered('x'.repeat(50), 20, theme)).length).toBe(3)
  })

  it('names the 256 colours past the sixteen', () => {
    expect(paletteColour(1, theme)).toBe('#ff0000')
    expect(paletteColour(196, theme)).toBe('#ff0000')
    expect(paletteColour(232, theme)).toBe('#080808')
  })
})
