/*
 * A unified diff, parsed into hunks and lines.
 *
 * Parsed rather than coloured by prefix, because the prefix lies: a line that
 * adds `++foo` starts with `++`, and a rule that reads two plus signs as a
 * header would draw the addition as one. The hunk header is the only line
 * that starts with `@@` *and* sits where a header sits.
 */

export type Row =
  | { readonly kind: 'add'; readonly text: string }
  | { readonly kind: 'del'; readonly text: string }
  | { readonly kind: 'same'; readonly text: string }

export interface Hunk {
  /** The `@@ … @@` line, verbatim. */
  readonly header: string
  readonly rows: readonly Row[]
}

export interface FileDiff {
  readonly path: string
  /** Where it came from, when git says it moved. */
  readonly from: string | null
  readonly hunks: readonly Hunk[]
  readonly binary: boolean
}

const PATH = /^(?:diff --git a\/(.+?) b\/(.+))$/

export function parse(diff: string): FileDiff[] {
  const files: FileDiff[] = []
  let path: string | null = null
  let from: string | null = null
  let binary = false
  let hunks: Hunk[] = []
  let rows: Row[] = []
  let header: string | null = null

  const closeHunk = (): void => {
    if (header !== null) hunks.push({ header, rows })
    header = null
    rows = []
  }
  const closeFile = (): void => {
    closeHunk()
    if (path !== null) files.push({ path, from, hunks, binary })
    path = null
    from = null
    binary = false
    hunks = []
  }

  for (const line of diff.split('\n')) {
    const named = PATH.exec(line)
    if (named) {
      closeFile()
      from = named[1]
      path = named[2]
      /* Same name on both sides means it did not move; `from` is only worth
         carrying when it says something. */
      if (from === path) from = null
      continue
    }
    if (path === null) continue

    if (line.startsWith('rename from ')) {
      from = line.slice('rename from '.length)
      continue
    }
    if (line.startsWith('rename to ')) {
      path = line.slice('rename to '.length)
      continue
    }
    if (line.startsWith('Binary files ')) {
      binary = true
      continue
    }
    if (line.startsWith('@@')) {
      closeHunk()
      header = line
      continue
    }
    /* Only inside a hunk is the first character a marker. Outside one it is
       part of `index`, `--- a/x`, `+++ b/x` and the other file headers. */
    if (header === null) continue

    if (line.startsWith('+')) rows.push({ kind: 'add', text: line.slice(1) })
    else if (line.startsWith('-')) rows.push({ kind: 'del', text: line.slice(1) })
    else if (line.startsWith(' ')) rows.push({ kind: 'same', text: line.slice(1) })
    /* `\ No newline at end of file` is a note about the hunk, not a line of
       it, and drawing it as context would put a backslash in the file. */
  }

  closeFile()
  return files
}

/* The two sides, for the side-by-side view. Deletions and additions inside one
   run line up; the shorter side is padded so the rows stay level. */
export function sides(hunk: Hunk): { left: (Row | null)[]; right: (Row | null)[] } {
  const left: (Row | null)[] = []
  const right: (Row | null)[] = []
  let at = 0

  while (at < hunk.rows.length) {
    const row = hunk.rows[at]
    if (row.kind === 'same') {
      left.push(row)
      right.push(row)
      at += 1
      continue
    }
    const dels: Row[] = []
    const adds: Row[] = []
    while (at < hunk.rows.length && hunk.rows[at].kind === 'del') dels.push(hunk.rows[at++])
    while (at < hunk.rows.length && hunk.rows[at].kind === 'add') adds.push(hunk.rows[at++])
    for (let i = 0; i < Math.max(dels.length, adds.length); i += 1) {
      left.push(dels[i] ?? null)
      right.push(adds[i] ?? null)
    }
  }

  return { left, right }
}

/* A line with its whitespace made visible: a space as `·`, a tab as `→`.
   Pieces rather than a string, so the marks can be drawn quieter than the
   text and the line keeps its width. */
export type Piece = { readonly text: string; readonly space: boolean }

export function visible(line: string): readonly Piece[] {
  const pieces: Piece[] = []
  for (const run of line.match(/[ \t]+|[^ \t]+/g) ?? []) {
    const space = run[0] === ' ' || run[0] === '\t'
    pieces.push({ text: space ? run.replace(/ /g, '·').replace(/\t/g, '→   ') : run, space })
  }
  return pieces
}

