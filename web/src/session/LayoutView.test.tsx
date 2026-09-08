import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { LayoutNode } from '../gen/bindings'
import { LayoutView } from './LayoutView'

vi.mock('./TerminalPane', () => ({
  TerminalPane: ({
    paneId,
    focused,
    onFocus
  }: {
    paneId: string
    focused: boolean
    onFocus: () => void
  }) => (
    <button type="button" data-focused={focused} data-testid={`pane-${paneId}`} onClick={onFocus}>
      {paneId}
    </button>
  )
}))

const leaf = (id: string): LayoutNode => ({
  type: 'leaf',
  id,
  tmuxTarget: `session:${id}`,
  kind: 'terminal',
  agent: 'none'
})

describe('LayoutView', () => {
  it('draws an Orca-style split and preserves its ratio', () => {
    const node: LayoutNode = {
      type: 'split',
      direction: 'horizontal',
      ratio: 0.35,
      first: leaf('left'),
      second: leaf('right')
    }

    const { container } = render(
      <LayoutView projectId="project" node={node} focusedId="right" onFocus={() => undefined} />
    )

    expect(screen.getByTestId('pty-split-horizontal')).toBeTruthy()
    expect(screen.getByTestId('pane-right').dataset.focused).toBe('true')
    const sides = container.querySelectorAll<HTMLElement>('.pty-split__side')
    expect(sides[0].style.flexGrow).toBe('35')
    expect(sides[1].style.flexGrow).toBe('65')
  })

  it('routes focus from the clicked leaf', () => {
    const focus = vi.fn()
    render(
      <LayoutView
        projectId="project"
        node={{
          type: 'split',
          direction: 'vertical',
          ratio: null,
          first: leaf('top'),
          second: leaf('bottom')
        }}
        focusedId="top"
        onFocus={focus}
      />
    )

    fireEvent.click(screen.getByTestId('pane-bottom'))
    expect(focus).toHaveBeenCalledWith('bottom')
  })
})
