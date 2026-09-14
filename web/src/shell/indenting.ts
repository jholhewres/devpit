/*
 * What Tab and Enter do inside the editor.
 *
 * Tab was moving focus, because that is what Tab does in a textarea and a
 * textarea is what this editor is. Keeping the field is worth a great deal —
 * selection, undo, composition and every accessibility affordance come free
 * and correct — so the keys it does not handle are handled here.
 *
 * Written as edits rather than as new text. An edit is a range and what
 * replaces it, which is exactly what `document.execCommand('insertText')`
 * takes, and that command is the only way to change a textarea without
 * throwing away the browser's own undo stack. Assigning `.value` would make
 * every indent the end of history.
 */

export interface Edit {
  /** The range to replace. */
  readonly from: number
  readonly to: number
  readonly insert: string
  /** Where the selection sits afterwards. */
  readonly select: readonly [number, number]
}

/** The openers that earn the next line a step in. */
const OPENS = ['{', '[', '(', ':']

/*
 * What one step in looks like in this file.
 *
 * Read from the file rather than configured, because the file already knows:
 * a Rust file indents by four and this one by two, and a setting would be a
 * question with a wrong answer half the time. A file that has not made up its
 * mind yet gets two spaces, which is this codebase's own.
 */
export function unit(text: string): string {
  let tabs = 0
  const widths: number[] = []
  for (const line of text.split('\n')) {
    if (line.startsWith('\t')) {
      tabs += 1
      continue
    }
    const spaces = line.length - line.trimStart().length
    if (spaces > 0 && line.trim().length > 0) widths.push(spaces)
  }
  if (tabs > widths.length) return '\t'
  // The smallest step anybody took, which is the step: a file indented by two
  // has plenty of lines at four and six, and none at one.
  const smallest = widths.reduce((least, wide) => Math.min(least, wide), Infinity)
  return ' '.repeat(Number.isFinite(smallest) ? Math.min(smallest, 8) : 2)
}

/** Where the line holding `at` begins. */
function lineStart(text: string, at: number): number {
  return text.lastIndexOf('\n', at - 1) + 1
}

/** The whitespace a line opens with. */
function leading(line: string): string {
  return line.slice(0, line.length - line.trimStart().length)
}

/** Whether the selection covers more than one line's worth of the text. */
function spansLines(text: string, from: number, to: number): boolean {
  return text.slice(from, to).includes('\n')
}

/** One step in, from where the cursor is. */
function stepIn(text: string, at: number, step: string): string {
  if (step === '\t') return step
  // To the next stop, not a fixed number of spaces: pressing Tab at column 3
  // with a step of two should reach column 4, not column 5.
  const column = at - lineStart(text, at)
  return ' '.repeat(step.length - (column % step.length))
}

/** The edit Tab makes, or nothing when the key should do what it always does. */
export function onTab(
  text: string,
  from: number,
  to: number,
  shifted: boolean,
  step: string,
): Edit | null {
  if (spansLines(text, from, to)) return shiftLines(text, from, to, shifted, step)
  if (!shifted) {
    const insert = stepIn(text, from, step)
    return { from, to, insert, select: [from + insert.length, from + insert.length] }
  }
  return outdentOne(text, from, step)
}

/** Every line the selection touches, moved one step in or out. */
function shiftLines(
  text: string,
  from: number,
  to: number,
  shifted: boolean,
  step: string,
): Edit {
  const start = lineStart(text, from)
  // Through the end of the last line the selection touches, so the whole of
  // it is one replacement and therefore one undo.
  const after = text.indexOf('\n', to)
  const end = after === -1 ? text.length : after
  const lines = text.slice(start, end).split('\n')

  const moved = lines.map((line) => {
    if (!shifted) {
      // An empty line gains nothing: whitespace on a line with nothing on it
      // is whitespace somebody has to delete later.
      return line.trim().length === 0 ? line : step + line
    }
    return line.slice(takeOff(line, step))
  })

  const insert = moved.join('\n')
  const grew = insert.length - (end - start)
  return {
    from: start,
    to: end,
    insert,
    /* Out to the whole of the first line, and on to the end of the last:
       the selection keeps the lines it had, so Tab can be pressed again, and
       it covers the indent it just put there rather than starting after it. */
    select: [start, to + grew],
  }
}

/** How much leading whitespace one outdent removes from this line. */
function takeOff(line: string, step: string): number {
  if (line.startsWith('\t')) return 1
  const spaces = leading(line).length
  return Math.min(spaces, step === '\t' ? 1 : step.length)
}

/** Shift-Tab with no selection: back out of the indent behind the cursor. */
function outdentOne(text: string, at: number, step: string): Edit | null {
  const start = lineStart(text, at)
  const before = text.slice(start, at)
  // Only in the indentation. Shift-Tab in the middle of a word is not an
  // outdent, and eating the characters there would be a surprise.
  if (before.trim().length > 0) return null
  const off = takeOff(before, step)
  if (off === 0) return null
  return { from: at - off, to: at, insert: '', select: [at - off, at - off] }
}

/**
 * The edit Enter makes: a new line that opens where this one did.
 *
 * Without it, every line after the first starts at column one and the person
 * types the indentation back by hand — which is the thing an editor is for.
 */
export function onEnter(text: string, from: number, to: number, step: string): Edit {
  const start = lineStart(text, from)
  const line = text.slice(start, from)
  const indent = leading(line)
  const opened = OPENS.includes(line.trimEnd().slice(-1)) ? step : ''
  const insert = `\n${indent}${opened}`
  return { from, to, insert, select: [from + insert.length, from + insert.length] }
}
