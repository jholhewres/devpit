import { act, cleanup, render } from '@testing-library/react'
import { createElement } from 'react'
import { describe, expect, it, vi } from 'vitest'

import { AGAIN, Checks, checksWords } from './Checks'

const asked = vi.fn()
vi.mock('../shell/live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    islandChecks: async () => {
      asked()
      return null
    },
  },
}))

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

/* Each ask goes to the code host, once a minute for as long as the island is open. */
describe('the checks on a session', () => {
  it('are not asked again while the window is hidden', async () => {
    vi.useFakeTimers()
    Object.defineProperty(document, 'hidden', { value: true, configurable: true })
    try {
      render(createElement(Checks, { sessionId: 's1' }))
      await act(async () => {
        await vi.advanceTimersByTimeAsync(AGAIN * 3)
      })
      expect(asked).toHaveBeenCalledTimes(1)
    } finally {
      Object.defineProperty(document, 'hidden', { value: false, configurable: true })
      vi.useRealTimers()
      cleanup()
    }
  })
})
