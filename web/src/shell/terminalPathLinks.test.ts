import type { IBufferLine } from '@xterm/xterm'
import { describe, expect, it } from 'vitest'

import { pathsIn } from './terminalLinks'
import { logicalLine } from './terminalPathLinks'

/** A buffer row of `cols` cells, from characters and their widths. */
function row(chars: Array<[string, number]>, cols: number, isWrapped = false): IBufferLine {
  const cells: Array<[string, number]> = []
  for (const [one, width] of chars) {
    cells.push([one, width])
    if (width === 2) cells.push(['', 0])
  }
  while (cells.length < cols) cells.push(['', 1])
  return {
    isWrapped,
    length: cols,
    getCell: (x: number) => {
      const cell = cells[x]
      return cell ? { getChars: () => cell[0], getWidth: () => cell[1] } : undefined
    },
  } as unknown as IBufferLine
}
const narrow = (text: string): Array<[string, number]> => [...text].map((one) => [one, 1])

describe('a path in the terminal, on screen', () => {
  it('sits in the cells after a wide character, not one to the left', () => {
    const lines = [row([['⏺', 2], ...narrow(' Read(/a/b.png)')], 30)]
    const read = logicalLine((y) => lines[y], 0, 30)
    const [path] = pathsIn(read.text)
    expect(path!.path).toBe('/a/b.png')
    expect(read.cells[path!.start]!.x).toBe(8)
  })

  it('is read whole across the rows the terminal wrapped it over', () => {
    const lines = [row(narrow('see /home/me/app/.playwr'), 24), row(narrow('ight-mcp/shot.jpeg'), 24, true)]
    for (const asked of [0, 1]) {
      const read = logicalLine((y) => lines[y], asked, 24)
      const [path] = pathsIn(read.text)
      expect(path!.path).toBe('/home/me/app/.playwright-mcp/shot.jpeg')
      expect(read.cells[path!.start]).toMatchObject({ x: 4, y: 0 })
      expect(read.cells[path!.end - 1]).toMatchObject({ x: 17, y: 1 })
    }
  })
})
