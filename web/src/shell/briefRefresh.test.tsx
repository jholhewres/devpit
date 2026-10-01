import { renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { useBriefRefresh } from './useBriefRefresh'

const refreshed = vi.fn(async (_id: string) => ({ status: 'ok', data: null }))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: { orchestratorRefresh: (id: string) => refreshed(id) },
}))

afterEach(() => vi.clearAllMocks())

describe("an orchestrator's brief", () => {
  it('is refreshed when an orchestrator comes in front, however it came', () => {
    const { rerender } = renderHook(({ project }) => useBriefRefresh(project), {
      initialProps: { project: { id: 'prj_api', orchestrator: null } as { id: string; orchestrator: string | null } | null },
    })
    expect(refreshed).not.toHaveBeenCalled()
    rerender({ project: { id: 'prj_orch', orchestrator: 'claude' } })
    expect(refreshed).toHaveBeenCalledWith('prj_orch')
  })

  it('is not asked again while the same orchestrator stays in front', () => {
    const { rerender } = renderHook(({ project }) => useBriefRefresh(project), {
      initialProps: { project: { id: 'prj_orch', orchestrator: 'claude' } as { id: string; orchestrator: string | null } | null },
    })
    rerender({ project: { id: 'prj_orch', orchestrator: 'claude' } })
    expect(refreshed).toHaveBeenCalledTimes(1)
    rerender({ project: null })
    expect(refreshed).toHaveBeenCalledTimes(1)
  })
})
