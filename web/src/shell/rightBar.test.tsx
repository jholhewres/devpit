import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { RightPanel } from './RightPanel'
import { Row } from './Row'

afterEach(cleanup)

const tree = {
  nodes: [
    { name: 'a', path: 'a', status: 'clean', children: null },
    { name: 'b', path: 'b', status: 'clean', children: null },
  ],
  changes: [] as unknown[],
  totals: { added: 0, removed: 0 },
  loading: false,
  error: null,
  version: 1,
  reload: vi.fn(),
}

vi.mock('./useTree', () => ({ useTree: () => tree }))
vi.mock('./useShell', () => ({
  useShell: () => ({ project: null, files: true, show: vi.fn() }),
}))
vi.mock('./useExplorerState', () => ({
  useExplorerState: () => ({
    view: 'tree',
    setView: vi.fn(),
    mode: 'names',
    setMode: vi.fn(),
    query: '',
    setQuery: vi.fn(),
  }),
}))
vi.mock('./live', () => ({
  ask: () => Promise.resolve({ data: null, error: null, loading: false }),
  commands: {},
}))

/*
 * Three words plus two counts took the whole width of a panel somebody has
 * just been given a handle to make narrower. The panel is the content, not its
 * own table of contents.
 */
describe('the panel picker', () => {
  it('says which panel each icon opens without spending the width on it', () => {
    render(<RightPanel onOpenFile={vi.fn()} />)
    for (const name of ['Explorer', 'Changes', 'History']) {
      const button = screen.getByLabelText(name)
      expect(button.getAttribute('title')).toBe(name)
      expect(button.textContent).not.toContain(name)
    }
  })

  it('shows a count only when there is something to count', () => {
    // A zero says the opposite of what the count is there for, and takes the
    // room to say it.
    render(<RightPanel onOpenFile={vi.fn()} />)
    expect(screen.getByLabelText('Explorer').textContent).toBe('2')
    expect(screen.getByLabelText('Changes').textContent).toBe('')
  })
})

describe('a row git is not watching', () => {
  it('recedes rather than disappearing', () => {
    // Still on disk and still worth opening — `target` and `node_modules` are
    // where you go looking for a build output. But a tree that draws them
    // exactly like `src` is a tree where the rows that matter are lost.
    const shown = render(
      <Row
        projectId="p"
        node={{ name: 'target', path: 'target', status: 'ignored', children: null }}
        version={1}
        depth={0}
        keep={new Set()}
        filtering={false}
        current={null}
        collapsed={0}
        onOpen={vi.fn()}
      />,
    )
    expect(shown.container.querySelector('.row')?.getAttribute('data-ignored')).toBe('true')
  })

  it('leaves a row git is watching alone', () => {
    const shown = render(
      <Row
        projectId="p"
        node={{ name: 'src', path: 'src', status: 'modified', children: null }}
        version={1}
        depth={0}
        keep={new Set()}
        filtering={false}
        current={null}
        collapsed={0}
        onOpen={vi.fn()}
      />,
    )
    expect(shown.container.querySelector('.row')?.hasAttribute('data-ignored')).toBe(false)
  })
})
