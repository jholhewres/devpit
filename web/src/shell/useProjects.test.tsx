import { act, renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import type { Project } from '../gen/bindings'

const project = (id: string): Project =>
  ({ id, name: id, rootPath: `/p/${id}`, group: null, accent: '#000000', worktrees: [], unreadable: null, orchestrator: null, origin: null, lastOpenedAt: null, icon: null, color: null }) as Project
const listed = { projects: [project('a'), project('b')] }
const projectList = vi.fn(async (_front: string | null) => listed)
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null }),
  commands: {
    projectList: (front: string | null) => projectList(front),
    projectOpen: async () => listed,
  },
}))
vi.mock('./window', () => ({ onEvent: () => () => undefined, inTauri: () => true }))

const { useProjects } = await import('./useProjects')

describe('the project list read again', () => {
  it("asks for the status of the project on screen, not every checkout's", async () => {
    const { result } = renderHook(() => useProjects())
    await waitFor(() => expect(result.current.projects.length).toBe(2))
    act(() => result.current.setProject('b'))
    act(() => result.current.reloadProjects())
    await waitFor(() => expect(projectList).toHaveBeenLastCalledWith('b'))
  })
})
