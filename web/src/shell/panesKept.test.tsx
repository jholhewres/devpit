import { act, cleanup, render } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { Panes } from './Panes'
import type { Tab } from './strip'

vi.mock('./BoardPane', () => ({ BoardPane: () => <div>board</div> }))
vi.mock('./TerminalPane', () => ({ TerminalPane: ({ tab }: { tab: Tab }) => <div>{tab.id}</div> }))

const term = (id: string): Tab => ({ id, kind: 'term' })
const shell: { open: Tab[]; active: Tab | null; project: { id: string; name: string } | null; [key: string]: unknown } = {
  open: [],
  active: null,
  show: vi.fn(),
  close: vi.fn(),
  project: null,
  projects: [],
}
vi.mock('./useShell', () => ({ useShell: () => shell }))

afterEach(cleanup)

const standIn = (project: string, tabs: Tab[]): void => {
  shell.project = { id: project, name: project }
  shell.open = tabs
  shell.active = tabs[0] ?? null
}

describe('the terminals kept across a project switch', () => {
  /* A terminal moved in the page takes its xterm's <style> out and back in,
     and WebKit then rebuilds every rule of the window. */
  it('are never moved in the page by a switch, there or back', () => {
    const a = [term('t-zeta')]
    const b = [term('t-alpha'), term('t-mid')]
    standIn('a', a)
    const { rerender } = render(<Panes />)
    const panes = document.querySelector('.panes')!
    const watch = new MutationObserver(() => {})
    watch.observe(panes, { childList: true })
    const moved: Node[] = []
    const go = (project: string, tabs: Tab[]): void => {
      standIn(project, tabs)
      act(() => rerender(<Panes />))
      for (const record of watch.takeRecords()) moved.push(...[...record.removedNodes].filter((node) => node.isConnected))
    }
    for (let round = 0; round < 3; round += 1) {
      go('b', b)
      go('a', a)
    }
    expect(document.querySelectorAll('[data-pane="term"]')).toHaveLength(3)
    expect(moved.map((node) => node.textContent)).toEqual([])
  })
})
