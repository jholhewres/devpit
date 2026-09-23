import type { ITheme } from '@xterm/xterm'

/*
 * A block's output, as lines of coloured runs.
 *
 * The output is what the program wrote, escape codes and all: colours, a
 * progress bar redrawn over itself with `\r`, a line cleared and written
 * again. Only a terminal reads that right, so a headless one does — at the
 * pane's width, so lines wrap where they wrapped — and the finished grid is
 * read back cell by cell into runs the page draws as text. Text, never
 * markup: nothing a program prints is ever parsed as HTML.
 */

export interface Run {
  readonly text: string
  readonly fg?: string
  readonly bg?: string
  readonly bold?: boolean
  readonly dim?: boolean
  readonly italic?: boolean
  readonly underline?: boolean
}

export type Line = readonly Run[]

/* The sixteen named colours, in the order the palette numbers them. */
const NAMED = [
  'black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white',
  'brightBlack', 'brightRed', 'brightGreen', 'brightYellow', 'brightBlue', 'brightMagenta', 'brightCyan', 'brightWhite',
] as const

const hex = (n: number): string => n.toString(16).padStart(2, '0')

/** A 256-colour index as a CSS colour: the theme's own sixteen, then the cube and the greys. */
export function paletteColour(index: number, theme: ITheme): string | undefined {
  if (index < 16) return theme[NAMED[index]!] as string | undefined
  if (index < 232) {
    const at = index - 16
    const level = (step: number): number => (step === 0 ? 0 : 55 + step * 40)
    return `#${hex(level(Math.floor(at / 36)))}${hex(level(Math.floor(at / 6) % 6))}${hex(level(at % 6))}`
  }
  const grey = 8 + (index - 232) * 10
  return `#${hex(grey)}${hex(grey)}${hex(grey)}`
}

interface Cell {
  getChars(): string
  getWidth(): number
  isFgRGB(): boolean
  isFgPalette(): boolean
  isBgRGB(): boolean
  isBgPalette(): boolean
  getFgColor(): number
  getBgColor(): number
  isBold(): number
  isDim(): number
  isItalic(): number
  isUnderline(): number
  isInverse(): number
}

function colour(rgb: boolean, palette: boolean, value: number, theme: ITheme): string | undefined {
  if (rgb) return `#${hex((value >> 16) & 0xff)}${hex((value >> 8) & 0xff)}${hex(value & 0xff)}`
  if (palette) return paletteColour(value, theme)
  return undefined
}

function styleOf(cell: Cell, theme: ITheme): Omit<Run, 'text'> {
  let fg = colour(cell.isFgRGB(), cell.isFgPalette(), cell.getFgColor(), theme)
  let bg = colour(cell.isBgRGB(), cell.isBgPalette(), cell.getBgColor(), theme)
  if (cell.isInverse()) {
    ;[fg, bg] = [bg ?? (theme.background as string | undefined), fg ?? (theme.foreground as string | undefined)]
  }
  return {
    ...(fg ? { fg } : {}),
    ...(bg ? { bg } : {}),
    ...(cell.isBold() ? { bold: true } : {}),
    ...(cell.isDim() ? { dim: true } : {}),
    ...(cell.isItalic() ? { italic: true } : {}),
    ...(cell.isUnderline() ? { underline: true } : {}),
  }
}

const same = (a: Omit<Run, 'text'>, b: Omit<Run, 'text'>): boolean =>
  a.fg === b.fg && a.bg === b.bg && a.bold === b.bold && a.dim === b.dim && a.italic === b.italic && a.underline === b.underline

/** The output drawn at `cols` columns, as lines of runs, without the blank rows after it. */
export async function rendered(output: string, cols: number, theme: ITheme): Promise<readonly Line[]> {
  const { Terminal } = await import('@xterm/headless')
  const term = new Terminal({ cols: Math.max(cols, 20), rows: 1, scrollback: 50_000, allowProposedApi: true, convertEol: false })
  await new Promise<void>((done) => term.write(output, done))
  const buffer = term.buffer.active
  const lines: Line[] = []
  for (let y = 0; y < buffer.length; y++) {
    const line = buffer.getLine(y)
    if (!line) continue
    const runs: { text: string; style: Omit<Run, 'text'> }[] = []
    for (let x = 0; x < line.length; x++) {
      const cell = line.getCell(x) as Cell | undefined
      if (!cell || cell.getWidth() === 0) continue
      const style = styleOf(cell, theme)
      const text = cell.getChars() || ' '
      const last = runs[runs.length - 1]
      if (last && same(last.style, style)) last.text += text
      else runs.push({ text, style })
    }
    /* Trailing blanks are the rest of the row, not something printed. */
    const lastRun = runs[runs.length - 1]
    if (lastRun && !lastRun.style.bg) lastRun.text = lastRun.text.replace(/\s+$/, '')
    const kept = runs.filter((one) => one.text.length > 0)
    /* A wrapped row continues the one before it: kept as its own line, since
       it was drawn as one at this width. */
    lines.push(kept.map(({ text, style }) => ({ text, ...style })))
  }
  term.dispose()
  while (lines.length > 0 && lines[lines.length - 1]!.length === 0) lines.pop()
  return lines
}

/** The output as plain text, escape codes and redraws resolved — what "copy output" copies. */
export function plain(lines: readonly Line[]): string {
  return lines.map((line) => line.map((run) => run.text).join('')).join('\n')
}
