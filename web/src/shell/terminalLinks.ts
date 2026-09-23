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

/* Ends where the extension does: a `/`, a word character or `@+-` right after
   it means the path goes on (`~/.config/devpit/x`, `node_modules/.bin/vite`),
   and a path that ends without an extension is not a link. */
const PATH =
  /(?:~\/|\/|\.{1,2}\/|(?<![\w/.~-])[\w.@-]+\/)[\w.@+\-/]*\.[A-Za-z0-9]{1,8}(?![\w/@+-])(?::(\d+))?/g
/** A URL, or a host written without its scheme: what is inside is not a file. */
const URL_SPAN = /\b[a-z][\w+.-]*:\/\/\S+|\bwww\.\S+/gi

export function pathsIn(text: string): PathLink[] {
  const urls = [...text.matchAll(URL_SPAN)].map((url) => [url.index, url.index + url[0].length] as const)
  const found: PathLink[] = []
  for (const match of text.matchAll(PATH)) {
    const whole = match[0]
    if (urls.some(([from, to]) => match.index < to && match.index + whole.length > from)) continue
    const lineNo = match[1] ? Number(match[1]) : null
    const path = lineNo === null ? whole : whole.slice(0, whole.lastIndexOf(':'))
    found.push({ start: match.index, end: match.index + whole.length, path, line: lineNo })
  }
  return found
}

/** A path as the app can open it: `~` expanded, project paths made relative
 *  (so they open editable in the project), the rest left absolute. */
export function openable(path: string, home: string | null, root: string | null, cwd: string | null): string {
  // `./src/a.ts` and `src/a.ts` are one file, and one tab.
  let full = path.replace(/^(?:\.\/)+/, '')
  if (full.startsWith('~/') && home) full = `${home}${full.slice(1)}`
  else if (!full.startsWith('/') && !full.startsWith('~/') && cwd) full = normalized(`${cwd.replace(/\/$/, '')}/${full}`)
  if (root && full.startsWith(`${root}/`)) return full.slice(root.length + 1)
  return full
}

/** `a/b/../c` as `a/c`: a path with `..` in it is refused by the file reader. */
function normalized(path: string): string {
  const parts: string[] = []
  for (const part of path.split('/')) {
    if (part === '..') parts.pop()
    else if (part !== '.') parts.push(part)
  }
  return parts.join('/')
}
