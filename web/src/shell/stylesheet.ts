import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'

/*
 * The shell's stylesheet as the browser sees it, for tests that read CSS.
 *
 * `shell.css` only imports its parts, in cascade order. Reading it alone would
 * read nothing, and a guard over nothing passes forever.
 */

/* From the project root rather than from `import.meta.url`: tests run under
   jsdom, where that is an http URL and `readFileSync` refuses it. */
export const SHEET = resolve(process.cwd(), 'src/shell/shell.css')

/** The files `shell.css` imports, in order. */
export function parts(sheet = SHEET): string[] {
  const text = readFileSync(sheet, 'utf8')
  return [...text.matchAll(/@import\s+'([^']+)';/g)].map((found) => resolve(dirname(sheet), found[1]!))
}

/** Every part, one after the other, as one sheet. */
export function stylesheet(sheet = SHEET): string {
  return parts(sheet)
    .map((part) => readFileSync(part, 'utf8'))
    .join('')
}

/** Classes a sheet styles that no source ever names — rules for markup that is gone. */
export function unusedIn(sheet: string, sources: readonly string[]): string[] {
  const css = sheet.replace(/\/\*[\s\S]*?\*\//g, '')
  const named = new Set([...css.matchAll(/\.([a-z][a-z0-9_-]*)/gi)].map((found) => found[1]!))
  const said = (name: string, source: string): boolean =>
    new RegExp(`(^|[^a-zA-Z0-9_-])${name}($|[^a-zA-Z0-9_-])`).test(source)
  return [...named].filter((name) => !sources.some((source) => said(name, source))).sort()
}
