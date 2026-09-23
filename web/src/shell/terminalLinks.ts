/*
 * File paths in a terminal's text, for a click to open them in the app.
 *
 * An agent prints `Read(/home/…/shot.png)`, a test prints `src/a.ts:12`, a
 * build prints where its log went. Only paths that name a file — a slash in
 * them and an extension at the end — so a word with a dot is not a link.
 * Absolute and `~/` paths, and project-relative ones.
 */
export interface PathLink {
  /** Where it starts and ends in the line, as string indexes. */
  readonly start: number
  readonly end: number
  readonly path: string
  /** The line after it, `:12`, when one was written. */
  readonly line: number | null
}

const PATH = /(?:~\/|\/|\.{1,2}\/|(?<![\w/.~-])[\w.@-]+\/)[\w.@+\-/]*\.[A-Za-z0-9]{1,8}(?::(\d+))?/g

export function pathsIn(text: string): PathLink[] {
  const found: PathLink[] = []
  for (const match of text.matchAll(PATH)) {
    const whole = match[0]
    const lineNo = match[1] ? Number(match[1]) : null
    const path = lineNo === null ? whole : whole.slice(0, whole.lastIndexOf(':'))
    // A URL's path is not a file here: `https://x.io/a.js` starts at `//`.
    if (match.index > 0 && text[match.index - 1] === ':') continue
    found.push({ start: match.index, end: match.index + whole.length, path, line: lineNo })
  }
  return found
}

/** A path as the app can open it: `~` expanded, project paths made relative
 *  (so they open editable in the project), the rest left absolute. */
export function openable(path: string, home: string | null, root: string | null, cwd: string | null): string {
  let full = path
  if (full.startsWith('~/') && home) full = `${home}${full.slice(1)}`
  else if (!full.startsWith('/') && cwd) full = `${cwd.replace(/\/$/, '')}/${full.replace(/^\.\//, '')}`
  if (root && full.startsWith(`${root}/`)) return full.slice(root.length + 1)
  return full
}
