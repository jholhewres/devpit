import { Terminal } from '@xterm/headless'
import { describe, expect, it } from 'vitest'

import { cutFlood, ground } from './floodCut'

const ROWS = 8
const COLS = 30
const SCROLLBACK = 40
const encode = (text: string): Uint8Array => new TextEncoder().encode(text)

const terminal = (): Terminal => new Terminal({ rows: ROWS, cols: COLS, scrollback: SCROLLBACK, allowProposedApi: true })
const written = (term: Terminal, data: Uint8Array): Promise<void> => new Promise((done) => term.write(data, done))

/* Everything a cut could change: every row kept, its cells' colours, the
   cursor, and the colour the next character would take. */
async function seen(term: Terminal): Promise<unknown> {
  await written(term, encode('Z'))
  const buffer = term.buffer.active
  const rows = []
  for (let y = 0; y < buffer.length; y += 1) {
    const line = buffer.getLine(y)!
    const cells = []
    for (let x = 0; x < COLS; x += 1) {
      const cell = line.getCell(x)!
      cells.push([cell.getChars(), cell.getFgColor(), cell.getBgColor(), cell.isBold(), cell.getWidth()])
    }
    rows.push([line.isWrapped, cells])
  }
  return { type: buffer.type, baseY: buffer.baseY, x: buffer.cursorX, y: buffer.cursorY, rows }
}

/* The same chunks into two terminals, one of them cut: they must end alike. */
async function bothWays(chunks: Uint8Array[]): Promise<{ whole: unknown; cut: unknown; taken: number }> {
  const whole = terminal()
  const cut = terminal()
  const stream = ground()
  let ring: Int32Array = new Int32Array(0)
  let taken = 0
  for (const chunk of chunks) {
    await written(whole, chunk)
    const done = cutFlood(chunk, stream, { rows: ROWS, scrollback: SCROLLBACK }, ring)
    ring = done.ring
    taken += done.cut
    await written(cut, done.bytes)
  }
  return { whole: await seen(whole), cut: await seen(cut), taken }
}

function chunked(bytes: Uint8Array, sizes: () => number): Uint8Array[] {
  const out: Uint8Array[] = []
  for (let at = 0; at < bytes.length; ) {
    const size = Math.max(1, sizes())
    out.push(bytes.subarray(at, at + size))
    at += size
  }
  return out
}

/* A small, seeded generator: the same streams every run. */
function random(seed: number): () => number {
  let state = seed >>> 0
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0
    return state / 2 ** 32
  }
}

describe('a flood cut down to what can still be seen', () => {
  it('leaves the terminal exactly as the whole of `yes` would, having cut most of it', async () => {
    const flood = encode('y\r\n'.repeat(5000))
    const { whole, cut, taken } = await bothWays(chunked(flood, () => 4096))
    expect(cut).toEqual(whole)
    expect(taken).toBeGreaterThan(flood.length / 2)
  })

  it('carries the colour the cut text left set', async () => {
    const lines = (text: string, n: number): string => `${text} \x1b[K\t\u4e2d\r\n`.repeat(n)
    const flood = encode(`\x1b[32m${lines('green', 300)}\x1b[0m\x1b[1;34m${lines('bold blue', 3000)}`)
    const { whole, cut, taken } = await bothWays([flood])
    expect(cut).toEqual(whole)
    expect(taken).toBeGreaterThan(0)
  })

  /* The cursor sent home over a full screen: the first lines after a cut go
     down through rows already written before any of them scrolls. */
  it('pushes out the rows the cursor walks down through before it scrolls', async () => {
    const flood = encode(`${'X'.repeat(COLS)}\r\n`.repeat(ROWS * 2) + '\x1b[H' + 'y\r\n'.repeat(2000))
    const { whole, cut, taken } = await bothWays([flood])
    expect(cut).toEqual(whole)
    expect(taken).toBeGreaterThan(0)
  })

  /* Moved up, the cursor writes over rows instead of scrolling them out: the
     line feeds a cut keeps are counted only past it. */
  it('keeps the line feeds a cut needs past a cursor moved up', async () => {
    const lines = (n: number): string => Array.from({ length: n }, (_, i) => `${i}\r\n`).join('')
    for (let after = 1; after <= SCROLLBACK + 2 * ROWS + 4; after += 1) {
      const { whole, cut, taken } = await bothWays([encode(`${lines(300)}\x1b[${ROWS}A${lines(after)}`)])
      expect(cut, `${after} lines after`).toEqual(whole)
      expect(taken).toBeGreaterThan(0)
    }
  })

  /* xterm parses a write after it is cut, and the pane can grow in between. */
  it('ends alike when the pane grows to its tallest before the cut write is parsed', async () => {
    const tallest = ROWS * 4
    const old = encode(`${'old '.repeat(7)}\r\n`.repeat(ROWS + SCROLLBACK))
    const flood = encode(Array.from({ length: 3000 }, (_, i) => `${i}\r\n`).join(''))
    const whole = terminal()
    const cut = terminal()
    await written(whole, old)
    await written(cut, old)
    const done = cutFlood(flood, ground(), { rows: ROWS, scrollback: SCROLLBACK, tallest })
    whole.resize(COLS, tallest)
    cut.resize(COLS, tallest)
    await written(whole, flood)
    await written(cut, done.bytes)
    expect(await seen(cut)).toEqual(await seen(whole))
    expect(done.cut).toBeGreaterThan(0)
  })

  /* A region set, or a buffer swapped, by a sequence a write ends inside of:
     read as one, they stop the cuts that would change rows outside it. */
  it('reads a sequence split between two writes at any byte as it reads it whole', async () => {
    /* Numbered, so a row written from the wrong place shows. */
    const lines = (n: number): string => Array.from({ length: n }, (_, i) => `${i}\r\n`).join('')
    const floods: [string, string, string][] = [
      [lines(200), '\x1b[3;6r', lines(200)],
      [`\x1b[3;6r${lines(200)}`, '\x1b[?1049h', `\x1b[r${lines(200)}\x1b[?1049l\x1b[H${lines(200)}`],
    ]
    for (const [before, sequence, after] of floods) {
      for (let at = 1; at < sequence.length; at += 1) {
        const chunks = [encode(before + sequence.slice(0, at)), encode(sequence.slice(at) + after)]
        const { whole, cut, taken } = await bothWays(chunks)
        expect(cut, `${JSON.stringify(sequence)} split at ${at}`).toEqual(whole)
        expect(taken).toBeGreaterThan(0)
      }
    }
  })

  /* Plain runs long enough to cut, broken by whatever a program prints. */
  it('ends alike for any mix of text and sequences, cut anywhere', async () => {
    const pieces = [
      'plain line\r\n',
      'more é中 text\r\n',
      '\x1b[31m',
      '\x1b[0m',
      '\x1b[1;38;5;200m',
      '\x1b[2K',
      '\r',
      '\t',
      '\n',
      '\x1b[A',
      '\x1b[2J',
      '\x1b[H',
      '\x1b[5;10r',
      '\x1b[r',
      '\x1b[?1049h',
      '\x1b[?1049l',
      '\x1b7',
      '\x1b8',
      '\x1b]2;a title\x07',
      '\x1bP1$r\x1b\\',
      '\u009b33m',
      '\x07',
      '\x18',
      '\x1b(0',
      '\x1b(B',
      '\x1bM',
      '\x1b[4h',
      '\x1b[4l',
    ]
    for (let seed = 1; seed <= Number(process.env.SEEDS ?? 30); seed += 1) {
      const next = random(seed)
      let text = ''
      for (let i = 0; i < 400; i += 1) {
        text += next() < 0.6 ? 'y\r\n'.repeat(1 + Math.floor(next() * 120)) : pieces[Math.floor(next() * pieces.length)]
      }
      const { whole, cut, taken } = await bothWays(chunked(encode(text), () => Math.floor(next() * 3000)))
      if (process.env.SEEDS) console.log(`seed ${seed} taken ${taken}`)
      expect(cut, `seed ${seed}`).toEqual(whole)
    }
  }, 60_000)
})
