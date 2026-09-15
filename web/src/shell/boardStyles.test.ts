import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import { SHEET, stylesheet, unusedIn } from './stylesheet'

/*
 * The board's styles live with the board.
 *
 * They had drifted into the onboarding and the bell parts, in the order they
 * were written. One cost was a real bug: `.tile { position: relative }` sat in
 * the bell part after `.tile--float { position: fixed }`, so the card in the
 * air was relative and drew nowhere near the pointer.
 */

const STYLES = resolve(SHEET, '../styles')
const part = (name: string): string => readFileSync(resolve(STYLES, name), 'utf8')

const sources = (): string[] =>
  (readdirSync(resolve(process.cwd(), 'src'), { recursive: true }) as string[])
    .filter((path) => path.endsWith('.tsx') && !path.endsWith('.test.tsx'))
    .map((path) => readFileSync(resolve(process.cwd(), 'src', path), 'utf8'))

describe('the board and card styles', () => {
  it('are not written in the onboarding or bell parts', () => {
    const strays = ['11-onboarding.css', '19-bell.css'].flatMap((name) =>
      part(name)
        .split('\n')
        .filter((line) => /^\.(blane|tile|cardp|cwork|crun|play|lstep)/.test(line))
        .map((line) => `${name}: ${line.slice(0, 40)}`),
    )
    expect(strays).toEqual([])
  })

  it('no longer carry rules for markup that is gone', () => {
    const sheet = stylesheet()
    for (const dead of ['.blane__agent', '.tile[aria-current', '.tile__run', '.tile__track', '.tile__elapsed', '.tile--enter']) {
      expect(sheet, dead).not.toContain(dead)
    }
  })

  it('style only classes some component names', () => {
    const sheet = part('07-board.css') + part('18-card.css')
    expect(unusedIn(sheet, sources())).toEqual([])
  })

  it('would catch a rule for a class nobody names', () => {
    expect(unusedIn('.tile { color: red }\n.tile__nothing { color: red }', ['<div className="tile" />'])).toEqual([
      'tile__nothing',
    ])
  })

  /* The float is the tile in the air: fixed, or it draws away from the pointer. */
  it('keeps the card in the air fixed', () => {
    const board = part('07-board.css')
    expect(board.indexOf('.tile--float {')).toBeGreaterThan(board.indexOf('.tile {'))
    expect(stylesheet().slice(stylesheet().indexOf('.tile--float {'))).not.toMatch(/\n\.tile \{[^}]*position:/)
  })
})
