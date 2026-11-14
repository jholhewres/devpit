import { act, cleanup, fireEvent, render, renderHook, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { rereadDetails, toggleDetail, useDetails } from './chatDetails'
import { ChatWhere } from './ChatWhere'

vi.mock('./useShell', () => ({
  useShell: () => ({ project: { name: 'devpit', worktrees: [{ branch: 'main' }] } }),
}))

afterEach(() => {
  cleanup()
  localStorage.clear()
  rereadDetails()
})

describe("a conversation's details", () => {
  it('all show until one is hidden, and a hidden one stays hidden after a restart', () => {
    const { result } = renderHook(() => useDetails())
    expect(['card', 'context', 'cost', 'project', 'machine', 'branch'].every((one) => result.current(one as never))).toBe(true)
    act(() => toggleDetail('cost'))
    expect(result.current('cost')).toBe(false)
    rereadDetails()
    const again = renderHook(() => useDetails())
    expect(again.result.current('cost')).toBe(false)
    expect(again.result.current('card')).toBe(true)
  })

  it('are picked from a right-click on the where line', () => {
    const { container } = render(<ChatWhere />)
    expect(screen.getByText('main')).toBeTruthy()
    fireEvent.contextMenu(container.querySelector('.composer__where')!)
    fireEvent.click(screen.getByRole('menuitem', { name: 'Branch' }))
    expect(screen.queryByText('main')).toBeNull()
    expect(screen.getByText('devpit')).toBeTruthy()
  })
})
