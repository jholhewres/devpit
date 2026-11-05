import { Terminal } from '@xterm/headless'
import { describe, expect, it } from 'vitest'

import { followAfterClear } from './terminalFollow'

const written = (term: Terminal, data: string): Promise<void> => new Promise((done) => term.write(data, done))

/* Claude Code's full redraw: screen and scrollback cleared, home, then more than a screen. */
const REDRAW = '\x1b[2J\x1b[3J\x1b[H' + 'new line\r\n'.repeat(30)

function terminal(): Terminal {
  const term = new Terminal({ rows: 10, cols: 40, scrollback: 1000, allowProposedApi: true })
  followAfterClear(term)
  return term
}

describe('a terminal whose program clears its scrollback', () => {
  it('follows the end again when the view was scrolled up, and keeps following new output', async () => {
    const term = terminal()
    await written(term, 'old line\r\n'.repeat(50))
    term.scrollLines(-5)
    await written(term, REDRAW)
    const { viewportY, baseY } = term.buffer.active
    expect(baseY).toBeGreaterThan(0)
    expect(viewportY).toBe(baseY)
    await written(term, 'more\r\n'.repeat(5))
    expect(term.buffer.active.viewportY).toBe(term.buffer.active.baseY)
  })

  it('follows once the redraw outgrows the screen, when it comes in a later write', async () => {
    const term = terminal()
    await written(term, 'old line\r\n'.repeat(50))
    term.scrollLines(-5)
    await written(term, '\x1b[2J\x1b[3J\x1b[H')
    await written(term, 'new line\r\n'.repeat(30))
    expect(term.buffer.active.viewportY).toBe(term.buffer.active.baseY)
  })

  it('leaves a view the person scrolled up where it is while nothing clears', async () => {
    const term = terminal()
    await written(term, 'old line\r\n'.repeat(50))
    term.scrollLines(-5)
    const reading = term.buffer.active.viewportY
    await written(term, 'more\r\n'.repeat(5))
    expect(term.buffer.active.viewportY).toBe(reading)
  })
})
