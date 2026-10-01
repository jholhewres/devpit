import { describe, expect, it } from 'vitest'

import { checksWords } from './Checks'

describe('the branch badge', () => {
  it('says the pull request and how its checks are doing', () => {
    expect(
      checksWords({ branch: 'fix/x', pull: { number: 12, state: 'OPEN', title: 'Fix', url: 'https://x' }, checks: 'failing' }),
    ).toBe('PR #12 open · checks failing')
  })

  it('says the branch when there is no pull request yet', () => {
    expect(checksWords({ branch: 'fix/x', pull: null, checks: 'running' })).toBe('fix/x · checks running')
  })
})
