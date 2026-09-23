import type { IBufferLine, IDisposable, Terminal } from '@xterm/xterm'

import { homeFolder } from './homeFolder'
import { wantLine } from './revealLine'
import type { Shell } from './shape'
import { openable, pathsIn } from './terminalLinks'

/*
 * File paths in a terminal, clickable: a click opens the file in the app —
 * a picture, a PDF, a log, JSON — in the viewer that reads it, at the line
 * when one was written after the path.
 */
export function linkPaths(
  terminal: Terminal,
  opening: { readonly current: { show: Shell['show']; root: string | null; cwd: string | null } },
): IDisposable {
  const home = homeFolder()
  return terminal.registerLinkProvider({
    provideLinks(y, answer) {
      const buffer = terminal.buffer.active
      const read = logicalLine((row) => buffer.getLine(row), y - 1, terminal.cols)
      answer(
        pathsIn(read.text)
          .map((one) => ({ one, from: read.cells[one.start], to: read.cells[one.end - 1] }))
          // Each row of a wrapped line is asked for; a path is answered once.
          .filter(({ from, to }) => from && to && from.y <= y - 1 && to.y >= y - 1)
          .map(({ one, from, to }) => ({
            range: { start: { x: from!.x + 1, y: from!.y + 1 }, end: { x: to!.x + to!.width, y: to!.y + 1 } },
            text: one.path,
            decorations: { underline: true, pointerCursor: true },
            activate: () => {
              const { show, root, cwd } = opening.current
              void home.then((found) => {
                const path = openable(one.path, found, root, cwd ?? root)
                show('file', { id: `file:${path}`, path, title: path.split('/').pop() })
                if (one.line) wantLine(path, one.line)
              })
            },
          })),
      )
    },
  })
}

/** Where a character of the joined text is on screen: its row, first cell, and
 *  how many cells it takes. */
export interface Cell {
  readonly x: number
  readonly y: number
  readonly width: number
}

/**
 * The whole line row `row` is part of — the rows before it and after it that
 * the terminal wrapped joined back — as text, with the cell each character of
 * that text sits at. A wide character takes two cells and one character, so a
 * string index is not a column.
 */
export function logicalLine(
  line: (row: number) => IBufferLine | undefined,
  row: number,
  cols: number,
): { text: string; cells: Cell[] } {
  let first = row
  while (first > 0 && line(first)?.isWrapped) first--
  let text = ''
  const cells: Cell[] = []
  for (let y = first; ; y++) {
    const one = line(y)
    if (!one) break
    for (let x = 0; x < cols; x++) {
      const cell = one.getCell(x)
      if (!cell) break
      const width = cell.getWidth()
      if (width === 0) continue
      const chars = cell.getChars() || ' '
      for (let i = 0; i < chars.length; i++) cells.push({ x, y, width })
      text += chars
    }
    if (!line(y + 1)?.isWrapped) break
  }
  const kept = text.trimEnd().length
  return { text: text.slice(0, kept), cells: cells.slice(0, kept) }
}
