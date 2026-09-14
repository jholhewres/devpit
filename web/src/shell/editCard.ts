import type { Row } from './diff'

/*
 * What an edit call did to a file, as diff rows.
 *
 * Built from the call's own arguments — `old_string` and `new_string` for an
 * edit, `content` for a write — because that is exactly what the agent asked
 * for, and it is there the moment the call is. Reading the file back would
 * show what is on disk now, which three edits later is something else.
 */

export interface EditDiff {
  /** As the agent wrote it, usually absolute. */
  readonly path: string
  readonly rows: readonly Row[]
  /** Lines left out to keep the card a card. */
  readonly hidden: number
}

/* Past this, a card stops being something you read in a thread. */
const MOST_ROWS = 120

/* A line diff is quadratic; past this, the new text is shown as written. */
const MOST_LINES_TO_ALIGN = 600

type Args = Record<string, unknown>

function parsed(input: string): Args | null {
  try {
    const value: unknown = JSON.parse(input)
    return typeof value === 'object' && value !== null ? (value as Args) : null
  } catch {
    return null
  }
}

const text = (value: unknown): string => (typeof value === 'string' ? value : '')

const lines = (value: string): string[] => (value === '' ? [] : value.replace(/\n$/, '').split('\n'))

/* The longest common subsequence of lines, walked back into rows. */
export function lineDiff(before: string, after: string): Row[] {
  const a = lines(before)
  const b = lines(after)
  if (a.length > MOST_LINES_TO_ALIGN || b.length > MOST_LINES_TO_ALIGN) {
    return [...a.map((line) => ({ kind: 'del' as const, text: line })), ...b.map((line) => ({ kind: 'add' as const, text: line }))]
  }
  const width = b.length + 1
  const table = new Uint32Array((a.length + 1) * width)
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) {
      table[i * width + j] =
        a[i] === b[j] ? table[(i + 1) * width + j + 1]! + 1 : Math.max(table[(i + 1) * width + j]!, table[i * width + j + 1]!)
    }
  }
  const rows: Row[] = []
  let i = 0
  let j = 0
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      rows.push({ kind: 'same', text: a[i]! })
      i++
      j++
    } else if (table[(i + 1) * width + j]! >= table[i * width + j + 1]!) {
      rows.push({ kind: 'del', text: a[i++]! })
    } else {
      rows.push({ kind: 'add', text: b[j++]! })
    }
  }
  while (i < a.length) rows.push({ kind: 'del', text: a[i++]! })
  while (j < b.length) rows.push({ kind: 'add', text: b[j++]! })
  return rows
}

/* The edit a call made, or null when it is not an edit this can read. */
export function editDiff(input: string): EditDiff | null {
  const args = parsed(input)
  if (!args) return null
  const path = text(args.file_path) || text(args.filePath) || text(args.path) || text(args.notebook_path)
  if (!path) return null

  let rows: Row[]
  if (Array.isArray(args.edits)) {
    rows = (args.edits as Args[]).flatMap((edit) => lineDiff(text(edit.old_string), text(edit.new_string)))
  } else if ('old_string' in args || 'new_string' in args) {
    rows = lineDiff(text(args.old_string), text(args.new_string))
  } else if ('content' in args) {
    // A write: what was there before is not in the call, so all of it is new.
    rows = lines(text(args.content)).map((line) => ({ kind: 'add' as const, text: line }))
  } else {
    return null
  }
  return { path, rows: rows.slice(0, MOST_ROWS), hidden: Math.max(0, rows.length - MOST_ROWS) }
}

/* What the card counts in its header. */
export function counted(rows: readonly Row[]): { added: number; removed: number } {
  return {
    added: rows.filter((row) => row.kind === 'add').length,
    removed: rows.filter((row) => row.kind === 'del').length,
  }
}
