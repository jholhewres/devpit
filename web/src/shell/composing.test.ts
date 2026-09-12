import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

/*
 * Nowhere may act on a raw Enter or Escape.
 *
 * Typing `á` is `'` then `a`, and the browser sends an **Enter** to commit the
 * composed character. A field that reads that as "done" submits halfway
 * through a word: the accent is lost and the form closes on a value nobody
 * finished typing. Escape is the mirror — an IME uses it to abandon what is
 * being composed, and a dialog that closes on it takes the whole form along.
 *
 * Every keyboard that composes an accent is affected, which is most of the
 * ones outside US English. It was found the way these are: somebody typed an
 * accented letter into a card's title.
 *
 * The fix is `typing.ts`. This is what stops the next field from being written
 * without it — the patch was nine files, and a tenth is one `onKeyDown` away.
 */

const SHELL = resolve(process.cwd(), 'src/shell')

/*
 * Where a raw comparison is right, with the reason.
 *
 * Only for keys an IME never sends, on elements that cannot be typed into.
 * A file is not allowed here; a line is — so adding a field to one of these
 * files still has to use the helper.
 */
const ALLOWED: readonly RegExp[] = [
  // A row or a tile, activated by Enter or Space. Not typeable, and Space is
  // not something a composition ever commits with.
  /\.key === 'Enter' \|\| \w+\.key === ' '/,
  // Escape-Escape to stop a turn, which is measured as two presses and never
  // reaches a text field.
  /\.key === 'Escape' &&/,
]

const sources = (): readonly string[] =>
  readdirSync(SHELL)
    .filter((name) => /\.tsx?$/.test(name))
    .filter((name) => !/\.test\.tsx?$/.test(name))
    .filter((name) => name !== 'typing.ts')

const raw = (text: string): string[] =>
  text
    .split('\n')
    /* Any name, not just `event`. The first version of this asked for
       `event.` literally, and its own falsification test caught that a field
       written with `(e) => e.key === 'Enter'` would walk straight past. */
    .filter((line) => /\b\w+\.key === '(Enter|Escape)'/.test(line))
    .filter((line) => !ALLOWED.some((allowed) => allowed.test(line)))

describe('Enter and Escape go through the composition guard', () => {
  it('has the guard to go through', () => {
    const typing = readFileSync(resolve(SHELL, 'typing.ts'), 'utf8')
    expect(typing).toMatch(/export function committed/)
    expect(typing).toMatch(/export function abandoned/)
  })

  it('is not bypassed anywhere in the shell', () => {
    const offenders = sources().flatMap((name) => {
      const lines = raw(readFileSync(resolve(SHELL, name), 'utf8'))
      return lines.map((line) => `${name}: ${line.trim()}`)
    })
    expect(offenders).toEqual([])
  })

  /* The guard only means anything if it can fail. */
  it('would catch a field written without it', () => {
    expect(raw("onKeyDown={(e) => e.key === 'Enter' && save()}")).toHaveLength(1)
    expect(raw("if (event.key === 'Escape') close()")).toHaveLength(1)
  })

  it('leaves the cases that are allowed alone', () => {
    expect(raw("if (event.key === 'Enter' || event.key === ' ') {")).toEqual([])
  })
})
