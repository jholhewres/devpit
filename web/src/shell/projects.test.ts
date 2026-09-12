import { describe, expect, it } from 'vitest'

import type { Project } from '../gen/bindings'
import { forgotten, found, remote, since, type Open } from './projects'

const project = (id: string): Project =>
  ({ id, name: id, rootPath: `/tmp/${id}`, group: null, accent: '#000', worktrees: [], unreadable: null, origin: null, lastOpenedAt: null })

const open = (ids: string[], current: string | null = ids[0] ?? null): Open => ({
  projects: ids.map(project),
  current,
})

describe('forgetting a project', () => {
  it('takes it out of the list', () => {
    expect(forgotten(open(['a', 'b', 'c']), 'b').projects.map((p) => p.id)).toEqual(['a', 'c'])
  })

  it('leaves you where you were when it was not the one you are in', () => {
    expect(forgotten(open(['a', 'b'], 'a'), 'b').current).toBe('a')
  })

  it('lands you on another one when it was', () => {
    expect(forgotten(open(['a', 'b'], 'a'), 'a').current).toBe('b')
  })

  it('leaves nothing to stand on when it was the last', () => {
    /* Which is what the onboarding is for: no project, nothing to show. */
    expect(forgotten(open(['a'], 'a'), 'a')).toEqual({ projects: [], current: null })
  })

  it('ignores an id that is not in the list', () => {
    const before = open(['a', 'b'], 'b')
    expect(forgotten(before, 'zz').projects).toHaveLength(2)
  })
})

describe('the project you are in', () => {
  it('is the one the id names', () => {
    expect(found(open(['a', 'b'], 'b'))?.name).toBe('b')
  })

  it('is nothing when the list is empty', () => {
    expect(found(open([], null))).toBeNull()
  })
})

describe('when a project was last opened', () => {
  /* A fixed clock, so a test does not start failing at midnight. */
  const now = Date.UTC(2026, 0, 20, 12, 0, 0)
  const ago = (seconds: number): number => now / 1000 - seconds

  it('says so plainly for a project that has never been opened', () => {
    /* The column exists to order the list. A row with no timestamp saying
       "just now" would put it at the top of a list it is at the bottom of. */
    expect(since(null, now)).toBe('never opened')
  })

  it('does not report a clock that runs fast as the future', () => {
    expect(since(ago(-30), now)).toBe('just now')
    expect(since(ago(5), now)).toBe('just now')
  })

  /* Against the same formatter rather than against English words: the
     wording is the reader's locale's business, and what this has to get
     right is the unit and the count. */
  const worded = (count: number, unit: Intl.RelativeTimeFormatUnit): string =>
    new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' }).format(-count, unit)

  it('picks the largest unit that fits', () => {
    expect(since(ago(3 * 60), now)).toBe(worded(3, 'minute'))
    expect(since(ago(5 * 3600), now)).toBe(worded(5, 'hour'))
    expect(since(ago(3 * 24 * 3600), now)).toBe(worded(3, 'day'))
    expect(since(ago(20 * 24 * 3600), now)).toBe(worded(2, 'week'))
    expect(since(ago(400 * 24 * 3600), now)).toBe(worded(1, 'year'))
  })
})

describe('the remote a project came from', () => {
  it('is nothing when there is none, rather than an empty line', () => {
    expect(remote(null)).toBeNull()
    expect(remote('   ')).toBeNull()
  })

  /* The same repository, written four ways. A list that showed these as four
     different strings would be answering a question nobody asked. */
  it('reads the same repository the same way however it was cloned', () => {
    const one = 'github.com/owner/name'
    expect(remote('https://github.com/owner/name.git')).toBe(one)
    expect(remote('git@github.com:owner/name.git')).toBe(one)
    expect(remote('ssh://git@github.com/owner/name')).toBe(one)
    expect(remote('https://token@github.com/owner/name/')).toBe(one)
  })
})
