import { renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import type { Project } from '../gen/bindings'

const boardGet = vi.fn(async (projectId: string) => ({ projectId, columns: [], cards: [] }))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null }),
  commands: { boardGet: (projectId: string) => boardGet(projectId) },
}))

const { useProjectBoards } = await import('./useProjectBoards')

const project = (id: string): Project =>
  ({ id, name: id, rootPath: `/p/${id}`, group: null, accent: '#000000', worktrees: [], unreadable: null, orchestrator: null, origin: null, lastOpenedAt: null, icon: null, color: null }) as Project

describe("the linked projects' boards", () => {
  it('are not read again when the same projects come back in a new list', async () => {
    const { rerender, result } = renderHook(({ list }) => useProjectBoards(list), {
      initialProps: { list: [project('a'), project('b')] },
    })
    await waitFor(() => expect(result.current.boards?.length).toBe(2))
    rerender({ list: [project('a'), project('b')] })
    rerender({ list: [project('a'), project('b')] })
    await new Promise((done) => setTimeout(done, 20))
    expect(boardGet).toHaveBeenCalledTimes(2)

    rerender({ list: [project('a'), project('b'), project('c')] })
    await waitFor(() => expect(result.current.boards?.length).toBe(3))
  })
})
