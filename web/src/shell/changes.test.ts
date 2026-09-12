import { describe, expect, it } from 'vitest'

import type { Change } from '../gen/bindings'
import { committable, discardLoses, grouped, stageable } from './changes'

const change = (path: string, staged: boolean, status: Change['status'] = 'modified'): Change =>
  ({ path, status, added: 1, removed: 0, staged })

describe('how the Changes panel is grouped', () => {
  const all = [
    change('a.rs', true),
    change('b.rs', false),
    change('c.rs', false, 'untracked'),
  ]

  it('puts what a commit would take in its own group', () => {
    expect(grouped(all).staged.map((one) => one.path)).toEqual(['a.rs'])
  })

  /* Collapsing untracked into changed is how a new file gets left out of a
     commit nobody noticed was missing it. */
  it('keeps untracked apart from changed', () => {
    expect(grouped(all).changed.map((one) => one.path)).toEqual(['b.rs'])
    expect(grouped(all).untracked.map((one) => one.path)).toEqual(['c.rs'])
  })

  it('counts a staged untracked file as staged, not as untracked', () => {
    const added = [change('new.rs', true, 'untracked')]
    expect(grouped(added).staged).toHaveLength(1)
    expect(grouped(added).untracked).toHaveLength(0)
  })
})

describe('what Stage All takes', () => {
  it('takes everything not already in the index', () => {
    expect(stageable([change('a.rs', true), change('b.rs', false)])).toEqual(['b.rs'])
  })
})

describe('whether committing does anything', () => {
  it('does nothing with nothing staged', () => {
    expect(committable([change('a.rs', false)], 'a message')).toBe(false)
  })

  it('does nothing with no message', () => {
    expect(committable([change('a.rs', true)], '   ')).toBe(false)
  })

  it('commits when something is staged and there is a message', () => {
    expect(committable([change('a.rs', true)], 'fix it')).toBe(true)
  })
})

describe('what discarding a change actually loses', () => {
  it('loses nothing when undoing a deletion — the file comes back as-is', () => {
    expect(discardLoses('deleted')).toBe('nothing')
  })

  it('loses the uncommitted edits when a modified file reverts to HEAD', () => {
    expect(discardLoses('modified')).toBe('edits')
  })

  it('loses the file itself when there is no earlier version to go back to', () => {
    expect(discardLoses('added')).toBe('file')
    expect(discardLoses('untracked')).toBe('file')
  })
})
