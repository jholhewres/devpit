import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import type { Terminal } from '@xterm/xterm'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { joinable, joined, type Strip, type Tab } from './strip'
import { TerminalMenu } from './TerminalMenu'

const term = (id: string, more: Partial<Tab> = {}): Tab => ({ id, kind: 'term', title: id, ...more })

afterEach(cleanup)

describe('joining a terminal tab into another', () => {
  it('drops the source without closing it and has the target read its tree again', () => {
    const was: Strip = { open: [term('a'), term('b'), term('c')], active: 'a' }
    const now = joined(was, 'a', 'c')
    expect(now.open.map((tab) => tab.id)).toEqual(['b', 'c'])
    expect(now.active).toBe('c')
    expect(now.open.find((tab) => tab.id === 'c')?.regrouped).toBe(1)
    expect(joined(now, 'b', 'c').open[0]?.regrouped).toBe(2)
  })

  it('leaves the strip alone when the target is gone', () => {
    const was: Strip = { open: [term('a')], active: 'a' }
    expect(joined(was, 'a', 'zz')).toBe(was)
  })

  it('offers only other plain terminal tabs', () => {
    const chat: Tab = { id: 'x', kind: 'chat' }
    const open = [term('a'), term('b'), term('card', { cardId: 'c1' }), chat]
    expect(joinable(open, 'a').map((tab) => tab.id)).toEqual(['b'])
  })

  it('offers nothing from a card tab or from something that is not a terminal', () => {
    const open = [term('a'), term('card', { cardId: 'c1' }), { id: 'x', kind: 'chat' } as Tab]
    expect(joinable(open, 'card')).toEqual([])
    expect(joinable(open, 'x')).toEqual([])
  })
})

describe("a terminal's right-click", () => {
  const terminal = { hasSelection: () => false, focus: vi.fn(), selectAll: vi.fn() } as unknown as Terminal
  const menu = (onSeparate?: () => void) =>
    render(
      <TerminalMenu
        at={{ x: 0, y: 0 }}
        terminal={terminal}
        projectId="p"
        onClose={vi.fn()}
        onFailed={vi.fn()}
        onClosePane={vi.fn()}
        onSeparate={onSeparate}
      />,
    )

  it('moves the pane to a tab of its own', () => {
    const separate = vi.fn()
    menu(separate)
    fireEvent.click(screen.getByRole('menuitem', { name: 'Move to New Tab' }))
    expect(separate).toHaveBeenCalledOnce()
  })

  it('does not offer it for the only pane of a tab', () => {
    menu()
    expect(screen.queryByRole('menuitem', { name: 'Move to New Tab' })).toBeNull()
  })
})
