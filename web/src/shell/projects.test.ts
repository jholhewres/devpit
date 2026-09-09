import { describe, expect, it } from 'vitest'

import type { Project } from '../gen/bindings'
import { forgotten, found, type Open } from './projects'

const project = (id: string): Project =>
  ({ id, name: id, rootPath: `/tmp/${id}`, group: null, accent: '#000', worktrees: [], unreadable: null })

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
