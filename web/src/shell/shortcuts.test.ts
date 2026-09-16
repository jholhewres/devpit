import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

import { SHORTCUTS, shortcutFor, type Press, type Shortcut } from './shortcuts'

/*
 * ⌘P was printed on the project button for months and nothing listened for
 * it. Nothing failed: a title is text, and text cannot be wrong on its own.
 * So the announcements are read off the screens here and held against the map.
 */

const SHELL = resolve(process.cwd(), 'src/shell')
const shipped = readdirSync(SHELL).filter(
  (name) => (name.endsWith('.tsx') || name.endsWith('.ts')) && !name.includes('.test.'),
)
const text = (name: string): string => readFileSync(resolve(SHELL, name), 'utf8')

/** Every shortcut a title announces, exactly as it is printed. */
function announced(): { file: string; combo: string }[] {
  const found: { file: string; combo: string }[] = []
  for (const file of shipped) {
    for (const match of text(file).matchAll(/title="[^"]*\(([⇧⌥⌘][^)]*)\)"/g)) {
      found.push({ file, combo: match[1]! })
    }
  }
  return found
}

describe('every shortcut the window announces', () => {
  it('is one the map knows', () => {
    const known = new Set(Object.values(SHORTCUTS))
    expect(announced().filter(({ combo }) => !known.has(combo))).toEqual([])
  })

  it('is announced somewhere at all, or this guard is checking nothing', () => {
    expect(announced().length).toBeGreaterThanOrEqual(4)
  })

  /* The other half of the drift: a key in the map that no screen reads. */
  it('has something that listens for it', () => {
    const listening = shipped
      .filter((name) => name !== 'shortcuts.ts')
      .map(text)
      .join('\n')
    const deaf = (Object.keys(SHORTCUTS) as Shortcut[]).filter(
      (name) => !listening.includes(`'${name}'`),
    )
    expect(deaf).toEqual([])
  })
})

describe('reading a press', () => {
  const press = (over: Partial<Press> = {}): Press => ({
    key: 'k',
    metaKey: false,
    ctrlKey: false,
    shiftKey: false,
    ...over,
  })

  it('needs ⌘, and takes ctrl in its place', () => {
    expect(shortcutFor(press())).toBeNull()
    expect(shortcutFor(press({ metaKey: true }))).toBe('palette')
    expect(shortcutFor(press({ ctrlKey: true }))).toBe('palette')
  })

  it('opens the project picker on ⌘P, however the key arrives', () => {
    expect(shortcutFor(press({ metaKey: true, key: 'p' }))).toBe('project')
    expect(shortcutFor(press({ metaKey: true, key: 'P' }))).toBe('project')
  })

  it('keeps the shifted pair apart from the plain ones', () => {
    expect(shortcutFor(press({ metaKey: true, key: 'd' }))).toBeNull()
    expect(shortcutFor(press({ metaKey: true, shiftKey: true, key: 'd' }))).toBe('splitRight')
    expect(shortcutFor(press({ metaKey: true, shiftKey: true, key: 'e' }))).toBe('splitDown')
    expect(shortcutFor(press({ metaKey: true, shiftKey: true, key: 'k' }))).toBeNull()
  })
})
