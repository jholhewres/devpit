import { renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { remember } from './tabs'
import { useTabs } from './useTabs'

vi.mock('./live', () => ({ ask: vi.fn(), commands: {} }))

const term = (id: string) => ({ id, kind: 'term' as const })

describe('switching project', () => {
  beforeEach(() => {
    localStorage.clear()
    remember('prj_a', { open: [term('tab_a')], active: 'tab_a' })
    remember('prj_b', { open: [term('tab_b')], active: 'tab_b' })
  })

  /* A terminal tab rendered under the wrong project asks that project for its
     layout, and the project makes it a shell nothing can reach again. */
  it('never renders one project with the other one’s tabs', () => {
    const seen: [string, string[]][] = []
    const { rerender } = renderHook(
      ({ projectId }: { projectId: string }) => {
        const tabs = useTabs(projectId)
        seen.push([projectId, tabs.open.map((tab) => tab.id)])
        return tabs
      },
      { initialProps: { projectId: 'prj_a' } },
    )
    rerender({ projectId: 'prj_b' })

    for (const [projectId, open] of seen) {
      expect(open).toEqual([projectId === 'prj_a' ? 'tab_a' : 'tab_b'])
    }
  })

  it('keeps what each project had open', () => {
    const { result, rerender } = renderHook(({ projectId }) => useTabs(projectId), {
      initialProps: { projectId: 'prj_a' },
    })
    rerender({ projectId: 'prj_b' })
    rerender({ projectId: 'prj_a' })
    expect(result.current.open.map((tab) => tab.id)).toEqual(['tab_a'])
  })
})
