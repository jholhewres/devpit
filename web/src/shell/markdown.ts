/*
 * Markdown, parsed to blocks rather than to HTML.
 *
 * Blocks and not a string of HTML on purpose: nothing here can produce markup
 * to inject, so raw HTML in the source is text and stays text. That is the
 * whole reason this is hand-written instead of pulled from a package.
 */

export type Block =
  | { readonly kind: 'heading'; readonly level: number; readonly text: string }
  | { readonly kind: 'paragraph'; readonly text: string }
  | { readonly kind: 'code'; readonly language: string; readonly text: string }
  | { readonly kind: 'list'; readonly ordered: boolean; readonly items: readonly string[] }
  | { readonly kind: 'quote'; readonly text: string }
  | { readonly kind: 'rule' }
  | {
      readonly kind: 'table'
      readonly head: readonly string[]
      readonly align: readonly Align[]
      readonly rows: readonly (readonly string[])[]
    }

export type Align = 'left' | 'center' | 'right' | null

/* A row's cells. `\|` is a pipe inside a cell, as GFM writes it — in code too,
   so `a\|b` in backticks is one cell. */
function cells(line: string): string[] {
  const inner = line.trim().replace(/^\|/, '').replace(/(?<!\\)\|$/, '')
  return inner.split(/(?<!\\)\|/).map((cell) => cell.trim().replace(/\\\|/g, '|'))
}

const DELIMITER = /^\s*:?-+:?\s*$/

/* A header row and the delimiter row under it, with as many cells as each
   other. Anything less is a paragraph that happens to contain a pipe. */
function tableAt(lines: readonly string[], at: number): string[] | null {
  const head = lines[at]
  const under = lines[at + 1]
  if (!head?.includes('|') || under === undefined || !under.includes('-')) return null
  const heads = cells(head)
  const marks = under.trim().replace(/^\|/, '').replace(/\|$/, '').split('|')
  if (marks.length !== heads.length || !marks.every((mark) => DELIMITER.test(mark))) return null
  return heads
}

function alignOf(mark: string): Align {
  const cell = mark.trim()
  if (cell.startsWith(':') && cell.endsWith(':')) return 'center'
  if (cell.endsWith(':')) return 'right'
  return cell.startsWith(':') ? 'left' : null
}

export function blocks(source: string): Block[] {
  const lines = source.split('\n')
  const out: Block[] = []
  let at = 0

  while (at < lines.length) {
    const line = lines[at]

    if (line.trim() === '') {
      at += 1
      continue
    }

    /* A fence runs to its closing fence, or to the end of the file: an
       unclosed fence is a fence, not a licence to parse the rest as markdown. */
    const fence = /^```(\w*)/.exec(line)
    if (fence) {
      const language = fence[1] ?? ''
      const body: string[] = []
      at += 1
      while (at < lines.length && !lines[at].startsWith('```')) {
        body.push(lines[at])
        at += 1
      }
      at += 1
      out.push({ kind: 'code', language, text: body.join('\n') })
      continue
    }

    const heading = /^(#{1,6})\s+(.*)$/.exec(line)
    if (heading) {
      out.push({ kind: 'heading', level: heading[1].length, text: heading[2].trim() })
      at += 1
      continue
    }

    if (/^(-{3,}|\*{3,}|_{3,})$/.test(line.trim())) {
      out.push({ kind: 'rule' })
      at += 1
      continue
    }

    if (/^>\s?/.test(line)) {
      const body: string[] = []
      while (at < lines.length && /^>\s?/.test(lines[at])) {
        body.push(lines[at].replace(/^>\s?/, ''))
        at += 1
      }
      out.push({ kind: 'quote', text: body.join('\n') })
      continue
    }

    const head = tableAt(lines, at)
    if (head) {
      const align = lines[at + 1].trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map(alignOf)
      const rows: string[][] = []
      at += 2
      while (at < lines.length && lines[at].trim() !== '' && lines[at].includes('|')) {
        const row = cells(lines[at])
        /* Every row as wide as the header: GFM pads a short row and drops what
           a long one has past the last column. */
        rows.push(head.map((_, column) => row[column] ?? ''))
        at += 1
      }
      out.push({ kind: 'table', head, align, rows })
      continue
    }

    const bullet = /^\s*([-*+]|\d+[.)])\s+/
    if (bullet.test(line)) {
      const ordered = /^\s*\d/.test(line)
      const items: string[] = []
      while (at < lines.length && bullet.test(lines[at])) {
        items.push(lines[at].replace(bullet, ''))
        at += 1
      }
      out.push({ kind: 'list', ordered, items })
      continue
    }

    const body: string[] = []
    while (
      at < lines.length &&
      lines[at].trim() !== '' &&
      !/^(#{1,6}\s|```|>|\s*[-*+]\s)/.test(lines[at]) &&
      // A table may start right under a line of prose, with no blank between.
      !(body.length > 0 && tableAt(lines, at))
    ) {
      body.push(lines[at])
      at += 1
    }
    out.push({ kind: 'paragraph', text: body.join('\n') })
  }

  return out
}

export type Span =
  | { readonly kind: 'text'; readonly text: string }
  | { readonly kind: 'code'; readonly text: string }
  | { readonly kind: 'strong'; readonly text: string }
  | { readonly kind: 'em'; readonly text: string }
  | { readonly kind: 'link'; readonly text: string; readonly href: string }
  | { readonly kind: 'image'; readonly alt: string; readonly src: string }

const INLINE =
  /(`[^`]+`)|(!\[[^\]]*\]\([^)]+\))|(\[[^\]]+\]\([^)]+\))|(\*\*[^*]+\*\*)|(\*[^*]+\*)/

/* One pass, longest markers first, so `**bold**` is not read as two italics. */
export function spans(text: string): Span[] {
  const out: Span[] = []
  let rest = text

  while (rest.length > 0) {
    const found = INLINE.exec(rest)
    if (!found || found.index === undefined) break
    if (found.index > 0) out.push({ kind: 'text', text: rest.slice(0, found.index) })
    const token = found[0]

    if (token.startsWith('`')) {
      out.push({ kind: 'code', text: token.slice(1, -1) })
    } else if (token.startsWith('![')) {
      const [, alt, src] = /^!\[([^\]]*)\]\(([^)]+)\)$/.exec(token) ?? []
      out.push({ kind: 'image', alt: alt ?? '', src: src ?? '' })
    } else if (token.startsWith('[')) {
      const [, label, href] = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(token) ?? []
      out.push({ kind: 'link', text: label ?? '', href: href ?? '' })
    } else if (token.startsWith('**')) {
      out.push({ kind: 'strong', text: token.slice(2, -2) })
    } else {
      out.push({ kind: 'em', text: token.slice(1, -1) })
    }

    rest = rest.slice(found.index + token.length)
  }

  if (rest.length > 0) out.push({ kind: 'text', text: rest })
  return out
}

/* Whether a link leaves the machine. Those open in the system browser; a
   relative one is a link inside the project and stays here. */
export const external = (href: string): boolean => /^[a-z][a-z0-9+.-]*:/i.test(href)

/* A path an answer names, as the file to open: against the folder the
   conversation runs in when it has one, which for a card is its checkout. */
export function inFolder(folder: string | null, here: string): string {
  if (!folder || here.startsWith('/')) return here
  return resolved(`${folder.replace(/\/$/, '')}/_`, here)
}

/* An image path in a markdown file is relative to the file, not to the
   project root — `./logo.png` in `docs/a/b.md` is `docs/a/logo.png`. */
export function resolved(fileP: string, src: string): string {
  if (external(src) || src.startsWith('/')) return src
  const folder = fileP.split('/').slice(0, -1)
  for (const part of src.split('/')) {
    if (part === '.' || part === '') continue
    if (part === '..') folder.pop()
    else folder.push(part)
  }
  return folder.join('/')
}
