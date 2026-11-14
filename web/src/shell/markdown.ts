/*
 * Markdown, parsed to blocks rather than to HTML.
 *
 * Blocks and not a string of HTML on purpose: nothing here can produce markup
 * to inject, so raw HTML in the source is text and stays text. That is the
 * whole reason this is hand-written instead of pulled from a package.
 */

import { urlsIn } from './terminalLinks'

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
  return read(source.split('\n'), 0).map((one) => one.block)
}

/* Where a block ended: the line reading goes on from, and the last line it looked at. */
type Read = { readonly block: Block; readonly next: number; readonly seen: number }

function read(lines: readonly string[], from: number): Read[] {
  const out: Read[] = []
  let at = from
  const push = (block: Block): void => {
    /* A fence, a heading or a rule stops at its own last line; the others
       stopped at the line after them, and past it when that line is not blank. */
    const own = block.kind === 'code' || block.kind === 'heading' || block.kind === 'rule'
    out.push({ block, next: at, seen: own ? at - 1 : lines[at]?.trim() === '' ? at : at + 1 })
  }

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
      push({ kind: 'code', language, text: body.join('\n') })
      continue
    }

    const heading = /^(#{1,6})\s+(.*)$/.exec(line)
    if (heading) {
      at += 1
      push({ kind: 'heading', level: heading[1].length, text: heading[2].trim() })
      continue
    }

    if (/^(-{3,}|\*{3,}|_{3,})$/.test(line.trim())) {
      at += 1
      push({ kind: 'rule' })
      continue
    }

    if (/^>\s?/.test(line)) {
      const body: string[] = []
      while (at < lines.length && /^>\s?/.test(lines[at])) {
        body.push(lines[at].replace(/^>\s?/, ''))
        at += 1
      }
      push({ kind: 'quote', text: body.join('\n') })
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
      push({ kind: 'table', head, align, rows })
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
      push({ kind: 'list', ordered, items })
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
    push({ kind: 'paragraph', text: body.join('\n') })
  }

  return out
}

/* A text parsed as it grows: an answer being written only ever gains text at
   its end, so the blocks closed before its last line are not read again. */
export type Parsed = {
  readonly source: string
  readonly blocks: readonly Block[]
  readonly read: readonly Read[]
  readonly lines: number
}

export function grown(was: Parsed | null, source: string): Parsed {
  if (!was || !source.startsWith(was.source)) {
    const lines = source.split('\n')
    /* Edited rather than grown: a block that reads the same is still the one drawn. */
    const all = read(lines, 0).map((one, at) => {
      const same = was?.blocks[at]
      return same && JSON.stringify(same) === JSON.stringify(one.block) ? { ...one, block: same } : one
    })
    return { source, blocks: all.map((one) => one.block), read: all, lines: lines.length }
  }
  /* Settled: every line it looked at came before the old last line, the only one that can still grow. */
  let kept = 0
  while (kept < was.read.length && was.read[kept]!.seen < was.lines - 1) kept += 1
  const from = kept > 0 ? was.read[kept - 1]!.next : 0
  const lines = source.split('\n')
  const all = [...was.read.slice(0, kept), ...read(lines, from)]
  return { source, blocks: [...was.blocks.slice(0, kept), ...all.slice(kept).map((one) => one.block)], read: all, lines: lines.length }
}

/* What a block being written shows: a bold or a code span it has opened and
   not yet closed is closed, so the marks do not flash as text until it is. */
export function mended(block: Block): Block {
  if (block.kind !== 'paragraph' && block.kind !== 'heading' && block.kind !== 'quote') return block
  const ticks = (block.text.match(/`/g) ?? []).length % 2 === 1
  const plain = ticks ? block.text : block.text.replace(/`[^`]*`/g, '')
  const stars = (plain.match(/\*\*/g) ?? []).length % 2 === 1
  if (!ticks && !stars) return block
  return { ...block, text: `${block.text}${ticks ? '`' : ''}${stars ? '**' : ''}` }
}

export type Span =
  | { readonly kind: 'text'; readonly text: string }
  | { readonly kind: 'code'; readonly text: string }
  | { readonly kind: 'strong'; readonly text: string }
  | { readonly kind: 'em'; readonly text: string }
  | { readonly kind: 'link'; readonly text: string; readonly href: string }
  | { readonly kind: 'image'; readonly alt: string; readonly src: string }
  | { readonly kind: 'math'; readonly text: string; readonly display: boolean }

const INLINE =
  /(`[^`]+`)|(!\[[^\]]*\]\([^)]+\))|(\[[^\]]+\]\([^)]+\))|(\*\*[^*]+\*\*)|(\*[^*]+\*)/

/* `$$…$$` and `\[…\]` on their own, `$…$` and `\(…\)` in a line. A single `$`
   opens only before a non-space and closes only after one, not before a digit,
   so "costs $5 and $10" stays prose. */
const MATH = /(\$\$[\s\S]+?\$\$)|(\\\[[\s\S]+?\\\])|(\\\([\s\S]+?\\\))|(\$(?=[^\s$])[^$\n]*?[^\s$\\]\$(?!\d))/
const WITH_MATH = new RegExp(`${MATH.source}|${INLINE.source}`)

/* One pass, longest markers first, so `**bold**` is not read as two italics.
   Maths is read only when asked: off, a `$` is the character it always was. */
export function spans(text: string, math = false): Span[] {
  const out: Span[] = []
  let rest = text

  while (rest.length > 0) {
    const found = (math ? WITH_MATH : INLINE).exec(rest)
    if (!found || found.index === undefined) break
    if (found.index > 0) out.push({ kind: 'text', text: rest.slice(0, found.index) })
    const token = found[0]

    if (math && /^(\$\$|\\\[)/.test(token)) {
      out.push({ kind: 'math', text: token.slice(2, -2).trim(), display: true })
    } else if (math && /^(\$|\\\()/.test(token)) {
      const inner = token.startsWith('$') ? token.slice(1, -1) : token.slice(2, -2)
      out.push({ kind: 'math', text: inner.trim(), display: false })
    } else if (token.startsWith('`')) {
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
  return out.flatMap((span) => (span.kind === 'text' ? linked(span.text) : [span]))
}

/* A bare address in the prose is a link too: agents write them plainly far
   more often than as `[text](url)`. */
function linked(text: string): Span[] {
  const found = urlsIn(text)
  if (found.length === 0) return [{ kind: 'text', text }]
  const out: Span[] = []
  let at = 0
  for (const one of found) {
    if (one.start > at) out.push({ kind: 'text', text: text.slice(at, one.start) })
    out.push({ kind: 'link', text: one.url, href: one.url })
    at = one.end
  }
  if (at < text.length) out.push({ kind: 'text', text: text.slice(at) })
  return out
}

/* Whether a link leaves the machine. Those open in the system browser; a
   relative one is a link inside the project and stays here. */
export const external = (href: string): boolean => /^[a-z][a-z0-9+.-]*:/i.test(href)

/* What a click on a link does: a web address opens in the browser, a path opens
   here, and any other scheme (`mailto:`, `file:`) is drawn as text, since
   nothing here opens it. */
export const linkTarget = (href: string): 'web' | 'file' | 'text' =>
  /^https?:\/\//i.test(href) ? 'web' : external(href) ? 'text' : 'file'

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
