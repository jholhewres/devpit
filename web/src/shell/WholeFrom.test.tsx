import { renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import { WholeContext } from './cutOutput'
import { useWholeOutput } from './WholeFrom'

const asked: unknown[][] = []
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    chatPartFull: (...args: unknown[]) => {
      asked.push(args)
      return { kind: 'tool_result', call_id: 'c1', output: '{"rows":[1,2,3]}', is_error: false, parent: null }
    },
  },
}))

const wrapper = ({ children }: { children: React.ReactNode }) => (
  <WholeContext.Provider value={{ projectId: 'p1', conversationId: 'v1' }}>{children}</WholeContext.Provider>
)

describe("the output an MCP tool's page gets", () => {
  it('is read back whole when the window got it cut, and is nothing until then', async () => {
    const cut = '{"rows":[1,\n… [cut: 9000 bytes in all]\n,3]}'
    const { result } = renderHook(() => useWholeOutput('m1', 'c1', cut), { wrapper })
    expect(result.current).toBeNull()
    await waitFor(() => expect(result.current).toBe('{"rows":[1,2,3]}'))
    expect(asked).toEqual([['p1', 'v1', 'm1', 'c1']])
  })

  it('is the output itself when it went whole, asked of nobody', () => {
    asked.length = 0
    const { result } = renderHook(() => useWholeOutput('m1', 'c1', '{"ok":true}'), { wrapper })
    expect(result.current).toBe('{"ok":true}')
    expect(asked).toEqual([])
  })
})
