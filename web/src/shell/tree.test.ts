import { describe, expect, it } from 'vitest'

import type { FileNode } from '../gen/bindings'
import { mark, matching, ordered, refreshChildren } from './tree'

const file = (name: string, path = name): FileNode =>
  ({ name, path, status: 'clean', children: null })
const dir = (name: string, children: FileNode[], path = name): FileNode =>
  ({ name, path, status: 'clean', children })

describe('the order of a tree', () => {
  it('puts folders before files', () => {
    const sorted = ordered([file('a.rs'), dir('zz', [])])
    expect(sorted.map((n) => n.name)).toEqual(['zz', 'a.rs'])
  })

  it('sorts each kind by name', () => {
    const sorted = ordered([file('b'), file('a'), dir('z', []), dir('y', [])])
    expect(sorted.map((n) => n.name)).toEqual(['y', 'z', 'a', 'b'])
  })

  it('does not mutate what it was given', () => {
    const given = [file('b'), file('a')]
    ordered(given)
    expect(given.map((n) => n.name)).toEqual(['b', 'a'])
  })
})

describe('the letter beside a path', () => {
  it('reads untracked as added', () => {
    expect(mark('untracked')).toBe('A')
  })

  it('says nothing for a clean file', () => {
    expect(mark('clean')).toBeNull()
  })
})

describe('filtering the tree', () => {
  const tree = [dir('src', [file('main.rs', 'src/main.rs'), file('lib.rs', 'src/lib.rs')], 'src')]

  it('keeps the file that matches', () => {
    expect(matching(tree, 'main')).toContain('src/main.rs')
  })

  it('keeps the folders on the way to it', () => {
    /* A match nobody can reach is a match nobody sees. */
    expect(matching(tree, 'main')).toContain('src')
  })

  it('leaves out what does not match', () => {
    expect(matching(tree, 'main').has('src/lib.rs')).toBe(false)
  })

  it('keeps nothing for an empty query, so the caller shows everything', () => {
    expect(matching(tree, '   ').size).toBe(0)
  })
})

describe('what a reload means for a folder already fetched once', () => {
  it('does nothing for a folder that was never opened', () => {
    expect(refreshChildren(false, false)).toBe('skip')
    expect(refreshChildren(false, true)).toBe('skip')
  })

  it('refreshes a folder that is open, since it is on screen', () => {
    expect(refreshChildren(true, true)).toBe('now')
  })

  /* Dropping rather than leaving it alone: without this, reopening a closed
     folder would show what it fetched before the reload, not what changed. */
  it('drops the cache for a closed folder, so reopening it fetches fresh', () => {
    expect(refreshChildren(true, false)).toBe('drop')
  })
})

describe('a path git is not watching', () => {
  it('gets no letter, because there is no change to report', () => {
    // `M`, `A` and `D` are things that happened. Nothing happened to an
    // ignored file as far as git is concerned.
    expect(mark('ignored')).toBeNull()
  })
})
