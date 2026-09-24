import { describe, expect, it } from 'vitest'

import { inFolder } from './markdown'

describe('a path an answer names', () => {
  it('is read in the folder the conversation runs in', () => {
    expect(inFolder('/home/me/.devpit/worktrees/p/card_1', 'docs/plan.md')).toBe('/home/me/.devpit/worktrees/p/card_1/docs/plan.md')
    expect(inFolder('/repo/', './a/../b.md')).toBe('/repo/b.md')
  })

  it('stays as it is when absolute, or when the conversation has no folder yet', () => {
    expect(inFolder('/repo', '/etc/hosts')).toBe('/etc/hosts')
    expect(inFolder(null, 'docs/plan.md')).toBe('docs/plan.md')
  })
})
