import { readdirSync, readFileSync } from 'node:fs'
import { basename, resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import { parts, SHEET } from './stylesheet'

const STYLES = resolve(SHEET, '../styles')

describe('the stylesheet in parts', () => {
  /* A part nobody imports is CSS that silently never applies. */
  it('imports every part exactly once, and nothing that is not there', () => {
    const imported = parts().map((part) => basename(part))
    const onDisk = readdirSync(STYLES).filter((name) => name.endsWith('.css'))
    expect([...imported].sort()).toEqual([...onDisk].sort())
    expect(new Set(imported).size).toBe(imported.length)
  })

  /* Order is the cascade, so the numbers in the names have to be the order. */
  it('imports the parts in the order their names give', () => {
    const imported = parts().map((part) => basename(part))
    expect(imported).toEqual([...imported].sort())
  })

  it('keeps rules out of the index, where they would win over every part', () => {
    const rules = readFileSync(SHEET, 'utf8')
      .replace(/\/\*[\s\S]*?\*\//g, '')
      .replace(/@import\s+'[^']+';/g, '')
      .trim()
    expect(rules).toBe('')
  })
})
