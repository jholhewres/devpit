import { describe, expect, it } from 'vitest'

import type { WorkspaceEntry } from '../gen/bindings'
import { crumbs, matching, meta, moved, parentOf, sorted } from './wsbrowse'

const entry = (over: Partial<WorkspaceEntry> & { name: string }): WorkspaceEntry => ({
  path: over.name,
  isDir: false,
  bytes: 0,
  modified: 0,
  count: null,
  ...over,
})

describe('crumbs', () => {
  it('reads a project folder by its name, not by its ULID', () => {
    /* The whole reason this file exists: every folder devpit makes is called
       `prj_01M24GHNGDMZCXRFWEDVK387KM`, and a trail spelling that out is a
       trail nobody can follow back. */
    const trail = crumbs('worktrees/prj_01M24G/card_01M2AY', {
      id: 'prj_01M24G',
      name: 'demos',
    })
    expect(trail.map((crumb) => crumb.label)).toEqual([
      'Workspace',
      'worktrees',
      'demos',
      'card_01M2AY',
    ])
  })

  it('keeps each crumb pointing at its own folder', () => {
    const trail = crumbs('projects/prj_1/sessions', null)
    expect(trail.map((crumb) => crumb.path)).toEqual([
      '',
      'projects',
      'projects/prj_1',
      'projects/prj_1/sessions',
    ])
  })

  it('is one crumb at the root', () => {
    expect(crumbs('', null)).toEqual([{ label: 'Workspace', path: '' }])
  })
})

describe('parentOf', () => {
  it('climbs one level, and stops at the root', () => {
    expect(parentOf('a/b/c')).toBe('a/b')
    expect(parentOf('a')).toBe('')
    // Null and empty are different: empty is the root, null is "no further".
    expect(parentOf('')).toBeNull()
  })
})

describe('sorted', () => {
  const rows = [
    entry({ name: 'zed.json', bytes: 10, modified: 300 }),
    entry({ name: 'alpha.json', bytes: 900, modified: 100 }),
    entry({ name: 'sessions', isDir: true, count: 4, modified: 200 }),
  ]

  it('puts folders first whatever the column', () => {
    for (const order of ['name', 'size', 'modified'] as const) {
      expect(sorted(rows, order)[0]?.name).toBe('sessions')
    }
  })

  it('orders names the way a person reads numbers', () => {
    const numbered = [entry({ name: 'part-100' }), entry({ name: 'part-9' })]
    expect(sorted(numbered, 'name').map((row) => row.name)).toEqual(['part-9', 'part-100'])
  })

  it('puts the biggest and the newest first', () => {
    expect(sorted(rows, 'size').map((row) => row.name)).toEqual([
      'sessions',
      'alpha.json',
      'zed.json',
    ])
    expect(sorted(rows, 'modified').map((row) => row.name)).toEqual([
      'sessions',
      'zed.json',
      'alpha.json',
    ])
  })

  it('does not reorder the array it was handed', () => {
    const given = [...rows]
    sorted(given, 'size')
    expect(given.map((row) => row.name)).toEqual(rows.map((row) => row.name))
  })
})

describe('matching', () => {
  it('ignores case and keeps everything when nothing is asked', () => {
    const rows = [entry({ name: 'Hooks.json' }), entry({ name: 'state.db' })]
    expect(matching(rows, 'hook').map((row) => row.name)).toEqual(['Hooks.json'])
    expect(matching(rows, '   ')).toHaveLength(2)
  })
})

describe('meta', () => {
  it('counts a folder and weighs a file', () => {
    expect(meta(entry({ name: 'x', isDir: true, count: 1 }))).toBe('1 item')
    expect(meta(entry({ name: 'x', isDir: true, count: 4 }))).toBe('4 items')
    // An empty folder says so rather than claiming zero of something.
    expect(meta(entry({ name: 'x', isDir: true, count: 0 }))).toBe('empty')
    expect(meta(entry({ name: 'x', bytes: 2048 }))).toBe('2 KB')
  })

  it('adds when it changed, when the disk said', () => {
    const now = Date.now()
    expect(meta(entry({ name: 'x', bytes: 1, modified: now }))).toContain('·')
    // Zero is not a date in 1970, it is "we could not read it".
    expect(meta(entry({ name: 'x', bytes: 1, modified: 0 }))).not.toContain('·')
  })
})

describe('moved', () => {
  it('starts at either end and stops at both', () => {
    expect(moved(-1, 1, 3)).toBe(0)
    expect(moved(-1, -1, 3)).toBe(2)
    expect(moved(2, 1, 3)).toBe(2)
    expect(moved(0, -1, 3)).toBe(0)
    expect(moved(0, 1, 0)).toBe(-1)
  })
})
