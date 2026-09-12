import { ranked } from './search'

/*
 * Name search over the flat index, the whole project rather than the nodes
 * a lazy tree happens to have loaded.
 *
 * Every mode reduces to one regex: literal text escaped for the plain and
 * whole-word cases, the query itself when the field says to treat it as
 * one. A broken pattern is reported, not thrown, so a stray `(` while
 * typing a real regex does not blank the screen.
 */

export interface FindFlags {
  readonly case: boolean
  readonly word: boolean
  readonly regex: boolean
}

export interface FindHit {
  readonly path: string
  readonly at: number
  readonly length: number
}

export interface FindResult {
  readonly hits: readonly FindHit[]
  readonly reason: string | null
}

const escape = (text: string): string => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

function locate(path: string, pattern: RegExp): FindHit | null {
  const found = pattern.exec(path)
  return found ? { path, at: found.index, length: found[0].length } : null
}

export function filter(paths: readonly string[], query: string, flags: FindFlags): FindResult {
  const needle = query.trim()
  if (!needle) return { hits: [], reason: null }

  const body = flags.regex ? needle : escape(needle)
  /* Grouped rather than `\\b${body}\\b`: an ungrouped boundary binds to one
     side of a regex `|`, so whole-word would silently stop applying to half
     of an alternation like `foo|bar`. */
  const withWord = flags.word ? `\\b(?:${body})\\b` : body
  let pattern: RegExp
  try {
    pattern = new RegExp(withWord, flags.case ? '' : 'i')
  } catch (thrown) {
    return { hits: [], reason: `Invalid regular expression: ${(thrown as Error).message}` }
  }

  const hits = paths.map((path) => locate(path, pattern)).filter((hit): hit is FindHit => hit !== null)

  /* A regex pattern rarely reads as a substring of what it matches, so
     ranking it against the raw query would score every hit at zero. Plain
     and whole-word hits always contain the query, so ranking still applies. */
  if (flags.regex) return { hits, reason: null }
  return { hits: ranked(hits, needle, (hit) => hit.path, hits.length), reason: null }
}

/* The index (apps/desktop/src/index.rs) drops every dotfile but `.github`,
   while the tree shows them all — so a query shaped like `.env` can come
   back empty for a file that is right there. This cannot be fixed here
   (crates/ and apps/desktop/ belong to another agent this round), so the
   empty state names the gap instead of leaving it looking like "not found". */
export function looksLikeDotfile(query: string): boolean {
  const name = query.trim().split('/').pop() ?? ''
  return name.startsWith('.')
}
