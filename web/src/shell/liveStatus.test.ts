import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import type { LiveSession } from '../gen/bindings'
import { inOrder, useLiveSessions } from './liveStatus'

const session = (name: string, status: string): LiveSession => ({ name, status }) as unknown as LiveSession

let answer = (): void => {}
const asked = vi.fn(() => new Promise<unknown>((resolve) => (answer = () => resolve({ data: { sessions: [] }, error: null }))))
vi.mock('./live', () => ({
  ask: (call: () => unknown) => call(),
  commands: { orchestratorSessions: () => asked() },
}))

describe('the sessions beside an orchestrator', () => {
  it('put the busy ones first, then go by name', () => {
    const listed = inOrder([session('b-idle', 'idle'), session('z-busy', 'busy'), session('a-idle', 'idle'), session('c-busy', 'busy')])
    expect(listed.map((one) => one.name)).toEqual(['c-busy', 'z-busy', 'a-idle', 'b-idle'])
  })

  /* tmux under load answers late; a read on every tick regardless piled up a
     waiting thread each. */
  it('are not read again on the clock while the last read is still out', async () => {
    vi.useFakeTimers()
    try {
      const { unmount } = renderHook(() => useLiveSessions('profile_1'))
      act(() => void vi.advanceTimersByTime(4000 * 3))
      expect(asked).toHaveBeenCalledTimes(1)

      await act(async () => answer())
      act(() => void vi.advanceTimersByTime(4000))
      expect(asked).toHaveBeenCalledTimes(2)
      unmount()
    } finally {
      vi.useRealTimers()
    }
  })
})
