import { describe, expect, it, vi } from 'vitest'

import { wired } from './fileMenu'
import type { Tab } from './strip'
import { tabMenu } from './tabMenu'

describe("the tab's menu", () => {
  const hands = () => ({ close: vi.fn(), sweep: vi.fn(), join: vi.fn() })
  const other: Tab = { id: 't2', kind: 'term', title: 'Terminal 2' }

  /* The guard checks that a handler exists, not that it calls anything. This
     is what catches an entry that closes the menu and does nothing. */
  it('every entry draws a rule or does something', () => {
    expect(wired(tabMenu(hands()))).toBe(true)
    expect(wired(tabMenu(hands(), [other]))).toBe(true)
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

  it('offers each tab it can join, and joins this one into it', () => {
    const acting = hands()
    tabMenu(acting, [other])
      .find((entry) => entry.label === 'Move into split with Terminal 2')
      ?.run?.('t1')
    expect(acting.join).toHaveBeenCalledWith('t1', 't2')
  })

  /* Orca offers Pin here, which this window does not have; and with no tab to
     join, Move to Split has nowhere to go. A control that cannot be wired
     leaves the screen. */
  it('offers nothing this window cannot do', () => {
    const labels = tabMenu(hands()).map((entry) => entry.label)
    expect(labels).not.toContain('Pin Tab')
    expect(labels.some((label) => label?.includes('split'))).toBe(false)
  })
})
