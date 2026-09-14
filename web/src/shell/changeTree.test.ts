import { describe, expect, it } from 'vitest'

import type { Change } from '../gen/bindings'
import { foldered, folders, paths, type Folder, type Node } from './changeTree'

const change = (path: string, over: Partial<Change> = {}): Change => ({
  path,
  status: 'modified',
  added: 1,
  removed: 0,
  staged: false,
  ...over,
})

/** The tree as indented lines, which is how it is read on screen. */
const drawn = (nodes: readonly Node[], depth = 0): string[] =>
  nodes.flatMap((node) =>
    node.kind === 'folder'
      ? [`${'  '.repeat(depth)}${node.name}/ ${node.files}`, ...drawn(node.children, depth + 1)]
      : [`${'  '.repeat(depth)}${node.name}`],
  )

describe('the changed files, as folders', () => {
  it('puts a file at the root on its own', () => {
    expect(drawn(foldered([change('vite.config.ts')]))).toEqual(['vite.config.ts'])
  })

  it('folds a run of folders with one child into one row', () => {
    // Three rows each containing only the next is depth, not information.
    expect(drawn(foldered([change('apps/desktop/src/main.rs')]))).toEqual([
      'apps/desktop/src/ 1',
      '  main.rs',
    ])
  })

  it('stops folding where the tree actually branches', () => {
    expect(
      drawn(foldered([change('apps/desktop/src/main.rs'), change('apps/desktop/src/steps/a.rs')])),
    ).toEqual(['apps/desktop/src/ 2', '  steps/ 1', '    a.rs', '  main.rs'])
  })

  it('does not fold a folder that holds a file and a folder', () => {
    // Two things is two rows, however few children the folder has.
    expect(drawn(foldered([change('a/one.rs'), change('a/b/two.rs')]))).toEqual([
      'a/ 2',
      '  b/ 1',
      '    two.rs',
      '  one.rs',
    ])
  })

  it('counts every file beneath a folder, not only its own', () => {
    const tree = foldered([
      change('a/b/one.rs'),
      change('a/b/two.rs'),
      change('a/c/three.rs'),
    ])
    expect((tree[0] as Folder).files).toBe(3)
  })

  it('puts folders above files, each by name', () => {
    expect(
      drawn(foldered([change('z.rs'), change('a.rs'), change('m/one.rs')])),
    ).toEqual(['m/ 1', '  one.rs', 'a.rs', 'z.rs'])
  })

  it('shows a file by its own name, not by its whole path', () => {
    const tree = foldered([change('deep/down/here/file.rs')])
    const file = (tree[0] as Folder).children[0]!
    expect(file.kind === 'file' && file.name).toBe('file.rs')
  })

  it('keeps the whole path on the change, because that is what git takes', () => {
    const tree = foldered([change('deep/file.rs')])
    const file = (tree[0] as Folder).children[0]!
    expect(file.kind === 'file' && file.change.path).toBe('deep/file.rs')
  })

  it('has nothing to say about nothing', () => {
    expect(foldered([])).toEqual([])
  })
})

describe('what a folder row can act on', () => {
  const tree = foldered([
    change('a/b/one.rs'),
    change('a/two.rs'),
    change('z.rs'),
  ])

  it('names every folder, so they can all be opened at once', () => {
    expect(folders(tree)).toEqual(['a', 'a/b'])
  })

  it('gives a folder\'s files their whole paths, for staging in one click', () => {
    expect(paths([tree[0]!]).sort()).toEqual(['a/b/one.rs', 'a/two.rs'])
  })

  it('counts a single file as itself', () => {
    expect(paths(tree)).toContain('z.rs')
  })
})
