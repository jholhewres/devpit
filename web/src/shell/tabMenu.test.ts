import { describe, expect, it, vi } from 'vitest'

import { wired } from './fileMenu'
import { tabMenu } from './tabMenu'

describe("the tab's menu", () => {
  const hands = () => ({ close: vi.fn(), sweep: vi.fn() })

  /* The guard checks that a handler exists, not that it calls anything. This
     is what catches an entry that closes the menu and does nothing. */
  it('every entry draws a rule or does something', () => {
    expect(wired(tabMenu(hands()))).toBe(true)
  })

  it('each close names the sweep it means', () => {
    const acting = hands()
    const menu = tabMenu(acting)
    const run = (label: string) => menu.find((entry) => entry.label === label)?.run?.('t1')

    run('Close')
    expect(acting.close).toHaveBeenCalledWith('t1')

    for (const [label, what] of [
      ['Close others', 'others'],
      ['Close to the right', 'right'],
      ['Close to the left', 'left'],
      ['Close all', 'all'],
    ] as const) {
      run(label)
      expect(acting.sweep).toHaveBeenCalledWith('t1', what)
    }
  })

  /* Orca offers Pin and Move to Split here. Neither exists in this window, and
     AGENTS.md says a control that cannot be wired leaves the screen. */
  it('offers nothing this window cannot do', () => {
    const labels = tabMenu(hands()).map((entry) => entry.label)
    expect(labels).not.toContain('Pin Tab')
    expect(labels.some((label) => label?.includes('Split'))).toBe(false)
  })
})
