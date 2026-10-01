import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import { useIslandOpens } from './useIslandOpens'

const heard = new Map<string, (payload: unknown) => void>()
vi.mock('./window', () => ({
  onCarried: (name: string, then: (payload: unknown) => void) => (heard.set(name, then), () => heard.delete(name)),
}))

describe('the island asking for a session', () => {
  it('opens a chat in its own project, once that project is on screen', () => {
    const setProject = vi.fn()
    const show = vi.fn()
    const { rerender } = renderHook(({ current }) => useIslandOpens(current, setProject, () => {}, show), {
      initialProps: { current: 'prj_a' },
    })
    act(() => heard.get('island:open-chat')?.(['prj_b', 'conv_1']))
    expect(setProject).toHaveBeenCalledWith('prj_b')
    expect(show).not.toHaveBeenCalled()
    rerender({ current: 'prj_b' })
    expect(show).toHaveBeenCalledWith('chat', { id: 'conv_1' })
  })

  it('opens an orchestrator by its project alone, its chat being the project', () => {
    const setProject = vi.fn()
    const show = vi.fn()
    renderHook(() => useIslandOpens('prj_a', setProject, () => {}, show))
    act(() => heard.get('island:open-chat')?.(['prj_orch', null]))
    expect(setProject).toHaveBeenCalledWith('prj_orch')
    expect(show).not.toHaveBeenCalled()
  })

  it('opens a terminal by its pane', () => {
    const openPane = vi.fn()
    renderHook(() => useIslandOpens('prj_a', () => {}, openPane, () => {}))
    act(() => heard.get('island:open-pane')?.(['prj_a', 'leaf_1']))
    expect(openPane).toHaveBeenCalledWith('leaf_1')
  })
})
