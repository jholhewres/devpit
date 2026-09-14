import type { Language } from './languages'

/*
 * Source text, as coloured runs.
 *
 * Tokens and never markup. `Markdown.tsx` makes the same promise and says why:
 * nothing here produces HTML, so a file containing `<script>` arrives as a
 * token whose text is `<script>` and is drawn as text. That is structural
 * rather than a sanitiser somebody has to keep ahead of — and it is the reason
 * this is written rather than installed, because every highlighter worth
 * installing hands back a string of HTML.
 *
 * One pass, left to right, no backtracking. It is a lexer: it finds comments,
 * strings, numbers and words. It does not know what anything means.
 */

export type Kind = 'comment' | 'string' | 'number' | 'word' | 'type' | 'property' | 'plain'

export interface Token {
  readonly text: string
  readonly kind: Kind
}

/*
 * Above this, the file is drawn plain.
 *
 * Measured, and the number comes from the expensive half rather than this one.
 * Tokenising is linear and cheap — 80KB of TypeScript in 2.2ms, 160KB in
 * 3.0ms. What costs is painting the result: the same 80KB re-rendered 1,446
 * spans in 18ms per keystroke, and 20KB in 9ms.
 *
 * That was jsdom, which is slower than any webview this ships in, so the real
 * figure is better and is not known. The ceiling is therefore set where the
 * slowest measurement still leaves room: the largest source file in this
 * repository is the 80KB of generated bindings, and this is comfortably above
 * it while capping the worst case at roughly a frame and a half in the worst
 * environment. A file past it is still editable — it is drawn in one colour,
 * which is what every editor did until recently.
 */
export const TOO_BIG = 128_000

const isDigit = (letter: string): boolean => letter >= '0' && letter <= '9'

const isWordStart = (letter: string): boolean =>
  (letter >= 'a' && letter <= 'z') ||
  (letter >= 'A' && letter <= 'Z') ||
  letter === '_' ||
  letter === '$' ||
  letter === '@'

const isWord = (letter: string): boolean => isWordStart(letter) || isDigit(letter)

/** The coloured runs of this text, merged so plain stretches are one token. */
export function tokens(text: string, language: Language | null): readonly Token[] {
  if (!language || text.length > TOO_BIG) {
    return text.length === 0 ? [] : [{ text, kind: 'plain' }]
  }

  const out: Token[] = []
  let plain = 0
  let at = 0

  const flush = (upTo: number): void => {
    if (upTo > plain) out.push({ text: text.slice(plain, upTo), kind: 'plain' })
  }
  const take = (from: number, to: number, kind: Kind): void => {
    flush(from)
    out.push({ text: text.slice(from, to), kind })
    plain = to
    at = to
  }

  while (at < text.length) {
    const here = text[at]!
    const began = at

    if (
      (language.line && text.startsWith(language.line, at)) ||
      (language.alsoLine && text.startsWith(language.alsoLine, at))
    ) {
      const ends = text.indexOf('\n', at)
      take(began, ends === -1 ? text.length : ends, 'comment')
      continue
    }

    if (language.block && text.startsWith(language.block[0], at)) {
      take(began, blockEnds(text, at, language), 'comment')
      continue
    }

    // A Rust `'` is a lifetime far more often than a character, and colouring
    // `'a` as an unterminated string paints the rest of the file.
    if (here === "'" && language.lifetimes) {
      const closes = charEnds(text, at)
      if (closes === null) {
        at += 1
        continue
      }
      take(began, closes, 'string')
      continue
    }

    if (language.quotes.includes(here)) {
      const ends = stringEnds(text, at, here)
      // `"name": "devpit"` is a name and a value, and drawing them the same
      // colour is what made a JSON file one flat wash. Only where the mark is
      // unambiguous: in JSON and YAML a quoted string before a colon is always
      // a key, which is not true of a TypeScript ternary.
      take(began, ends, keyAhead(text, ends, language) ? 'property' : 'string')
      continue
    }

    if (isDigit(here)) {
      take(began, numberEnds(text, at), 'number')
      continue
    }

    if (isWordStart(here)) {
      let to = at + 1
      while (to < text.length && isWord(text[to]!)) to += 1
      const word = text.slice(at, to)
      if (language.words.has(word)) take(began, to, 'word')
      else if (language.types?.has(word)) take(began, to, 'type')
      else if (language.bareKeys && keyAhead(text, to, language)) take(began, to, 'property')
      else at = to
      continue
    }

    at += 1
  }

  flush(text.length)
  return out
}

/** Whether what follows is the mark that made the token before it a key. */
function keyAhead(text: string, from: number, language: Language): boolean {
  if (!language.keys) return false
  let at = from
  while (at < text.length && (text[at] === ' ' || text[at] === '\t')) at += 1
  return text[at] === language.keys
}

/** Where a block comment ends, counting depth when the language nests. */
function blockEnds(text: string, from: number, language: Language): number {
  const [opens, closes] = language.block!
  let depth = 1
  let at = from + opens.length
  while (at < text.length) {
    if (language.nests && text.startsWith(opens, at)) {
      depth += 1
      at += opens.length
      continue
    }
    if (text.startsWith(closes, at)) {
      depth -= 1
      at += closes.length
      if (depth === 0) return at
      continue
    }
    at += 1
  }
  return text.length
}

/*
 * Where a string ends.
 *
 * An unterminated one ends at the newline rather than running to the end of
 * the file — a quote somebody is halfway through typing must not repaint
 * everything below it. A backtick is the exception, because it is the one
 * quote that means to span lines.
 */
function stringEnds(text: string, from: number, quote: string): number {
  let at = from + 1
  while (at < text.length) {
    const here = text[at]!
    if (here === '\\') {
      at += 2
      continue
    }
    if (here === quote) return at + 1
    if (here === '\n' && quote !== '`') return at
    at += 1
  }
  return text.length
}

/** Where a character literal ends, or nothing when this `'` is a lifetime. */
function charEnds(text: string, from: number): number | null {
  let at = from + 1
  if (text[at] === '\\') at += 2
  else at += 1
  return text[at] === "'" ? at + 1 : null
}

/** Where a number ends. Hex, underscores and exponents are all still it. */
function numberEnds(text: string, from: number): number {
  let at = from
  while (at < text.length) {
    const here = text[at]!
    const exponent =
      (here === '+' || here === '-') && (text[at - 1] === 'e' || text[at - 1] === 'E')
    if (isWord(here) || here === '.' || exponent) at += 1
    else break
  }
  return at
}
