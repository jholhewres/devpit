import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

/**
 * The rule that makes a column the person's rather than the product's.
 *
 * A screen that matches on "review" breaks the first time someone renames that
 * lane, and renaming is the point. The seeded names live in one constant on
 * the Rust side and nowhere else; this asserts the frontend never learned them.
 */
describe('columns belong to the person', () => {
  const SEEDED = ['inbox', 'refine', 'review', 'doing', 'check', 'ship']

  /**
   * Comments are not code. The prose explaining this rule quotes a column
   * name, and a check that cannot tell the two apart fails on its own
   * documentation — which is how a guard gets deleted rather than obeyed.
   */
  function codeOf(file: string): string {
    return readFileSync(join(__dirname, file), 'utf8')
      .replace(/\/\*[\s\S]*?\*\//g, '')
      .split('\n')
      .map((line) => line.split('//')[0])
      .join('\n')
  }

  it('no screen matches on a seeded column name', () => {
    const source = codeOf('BoardSurface.tsx')
    for (const name of SEEDED) {
      expect(source).not.toContain(`'${name}'`)
      expect(source).not.toContain(`"${name}"`)
    }
  })

  /**
   * The other half of the same rule: the board draws whatever columns come
   * back, in the order they come back, so a renamed or reordered lane needs no
   * change here.
   */
  it('the lanes are drawn from the response, not from a list in the code', () => {
    const source = codeOf('BoardSurface.tsx')
    expect(source).toContain('current.columns.map')
  })
})

/**
 * A deploy has no undo, so it is not fired by dropping a card on a lane.
 */
describe('an irreversible step is confirmed', () => {
  it('the drop asks before sending confirmed', () => {
    const source = readFileSync(join(__dirname, 'BoardSurface.tsx'), 'utf8')
    expect(source).toContain('column.step?.irreversible')
    expect(source).toContain('window.confirm')
  })
})
