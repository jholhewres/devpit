import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import { useTrayOpens } from './useTrayOpens'

const heard = new Map<string, (payload: unknown) => void>()
vi.mock('./window', () => ({
  onCarried: (name: string, then: (payload: unknown) => void) => (heard.set(name, then), () => heard.delete(name)),
}))

const opens = () => ({ setProject: vi.fn(), show: vi.fn(), openCard: vi.fn(), openPrefs: vi.fn() })

describe('the tray asking for something', () => {
  it('opens a card on the board of its own project', () => {
    const to = opens()
    renderHook(() => useTrayOpens('prj_a', to.setProject, to.show, to.openCard, to.openPrefs))
    act(() => heard.get('tray:open-card')?.(['prj_b', 'card_1']))
    expect(to.setProject).toHaveBeenCalledWith('prj_b')
    expect(to.show).toHaveBeenCalledWith('board')
    expect(to.openCard).toHaveBeenCalledWith('card_1')
  })

  it('opens a project, and leaves the one on screen alone', () => {
    const to = opens()
    renderHook(() => useTrayOpens('prj_a', to.setProject, to.show, to.openCard, to.openPrefs))
    act(() => heard.get('tray:open-project')?.('prj_a'))
    expect(to.setProject).not.toHaveBeenCalled()
    act(() => heard.get('tray:open-project')?.('prj_b'))
    expect(to.setProject).toHaveBeenCalledWith('prj_b')
  })

  it('opens a new terminal, and Settings', () => {
    const to = opens()
    renderHook(() => useTrayOpens('prj_a', to.setProject, to.show, to.openCard, to.openPrefs))
    act(() => heard.get('tray:new-terminal')?.(null))
    expect(to.show).toHaveBeenCalledWith('term')
    act(() => heard.get('tray:settings')?.(null))
    expect(to.openPrefs).toHaveBeenCalledWith('general')
  })
})
