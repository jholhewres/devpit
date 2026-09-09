import { describe, expect, it } from 'vitest'

import { forgotten } from './projects'

const open = (projects: string[], current = projects[0] ?? '') => ({ projects, current })

describe('forgetting a project', () => {
  it('takes it out of the list', () => {
    expect(forgotten(open(['a', 'b', 'c']), 'b').projects).toEqual(['a', 'c'])
  })

  it('leaves you where you were when it was not the one you are in', () => {
    expect(forgotten(open(['a', 'b'], 'a'), 'b').current).toBe('a')
  })

  it('lands you on another one when it was', () => {
    expect(forgotten(open(['a', 'b'], 'a'), 'a')).toEqual({ projects: ['b'], current: 'b' })
  })

  it('leaves nothing to stand on when it was the last', () => {
    /* Which is what the onboarding is for: no project, nothing to show. */
    expect(forgotten(open(['a'], 'a'), 'a')).toEqual({ projects: [], current: '' })
  })

  it('ignores a name that is not in the list', () => {
    expect(forgotten(open(['a', 'b'], 'b'), 'zz')).toEqual({ projects: ['a', 'b'], current: 'b' })
  })
})
