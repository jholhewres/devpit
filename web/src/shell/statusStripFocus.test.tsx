import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Tab } from './strip'
import { tabOfPane } from './strip'
import { StatusStrip } from './StatusStrip'

afterEach(cleanup)

const tabs: Tab[] = [
  { id: 'term_1', kind: 'term', panes: ['leaf_1'] },
  // A split tab: two panes, one tab.
  { id: 'term_2', kind: 'term', panes: ['leaf_2', 'leaf_3'] },
  { id: 'board', kind: 'board' },
]

describe('the tab a pane is drawn in', () => {
  it('is the tab listing it, split or not', () => {
    expect(tabOfPane(tabs, 'leaf_1')).toBe('term_1')
    expect(tabOfPane(tabs, 'leaf_3')).toBe('term_2')
  })

  it('is nothing for a pane no open tab shows', () => {
    expect(tabOfPane(tabs, 'leaf_9')).toBeNull()
  })
})

const focus = vi.fn()
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'p' }, focus, open: tabs }) }))
vi.mock('./useUsage', () => ({
  useUsage: () => ({ memoryKb: 1024, cpuTenths: 5, proportional: true, panes: [{ paneId: 'leaf_3' }] }),
}))
/* The monitor itself is tested on its own; here only its Show matters. */
vi.mock('./Monitor', () => ({
  Monitor: ({ onShow }: { onShow: (paneId: string) => void }) => <button onClick={() => onShow('leaf_3')}>Show leaf_3</button>,
}))

describe('showing a terminal from the resource strip', () => {
  it('focuses the tab that holds the pane, not the pane id', () => {
    // Before, `focus('leaf_3')` was called — an id no tab has.
    render(<StatusStrip />)
    fireEvent.click(screen.getByRole('button', { expanded: false }))
    fireEvent.click(screen.getByText('Show leaf_3'))
    expect(focus).toHaveBeenCalledWith('term_2')
    expect(focus).not.toHaveBeenCalledWith('leaf_3')
  })
})
