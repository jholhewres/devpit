import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { PaneCorner } from './PaneCorner'
import type { Tab } from './strip'

const shell = { show: vi.fn(), close: vi.fn(), openCard: vi.fn(), open: [] as Tab[] }
vi.mock('./useShell', () => ({ useShell: () => shell }))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  shell.open = []
})

describe('the icons in a pane corner', () => {
  it('opens a new chat and a new terminal, and closes its own tab', () => {
    render(<PaneCorner tabId="conv_1" what="chat" />)
    fireEvent.click(screen.getByLabelText('New chat'))
    fireEvent.click(screen.getByLabelText('New terminal'))
    fireEvent.click(screen.getByLabelText('Close chat'))
    expect(shell.show.mock.calls).toEqual([['chat'], ['term']])
    expect(shell.close).toHaveBeenCalledWith('conv_1')
  })

  /* A chat has no tree to split, so it is not offered a split it cannot do. */
  it('offers splitting only to a pane that can split', () => {
    render(<PaneCorner tabId="conv_1" what="chat" />)
    expect(screen.queryByLabelText('Split right')).toBeNull()
    cleanup()

    const onSplit = vi.fn()
    render(<PaneCorner tabId="term_1" what="terminal" onSplit={onSplit} />)
    fireEvent.click(screen.getByLabelText('Split right'))
    fireEvent.click(screen.getByLabelText('Split down'))
    // The same directions the shortcuts send: right is horizontal, down is vertical.
    expect(onSplit.mock.calls).toEqual([['horizontal'], ['vertical']])
  })

  it('keeps what the pane adds of its own beside the actions', () => {
    render(
      <PaneCorner tabId="conv_1" what="chat">
        <span>$0.42</span>
      </PaneCorner>,
    )
    expect(screen.getByText('$0.42')).toBeTruthy()
  })

  it('goes back to the card a terminal was opened for, and only from such a terminal', () => {
    shell.open = [
      { id: 'tab_card', kind: 'term', title: 'Wire the board', cardId: 'card_1' },
      { id: 'tab_plain', kind: 'term', title: 'zsh' },
    ]
    render(<PaneCorner tabId="tab_plain" what="terminal" />)
    expect(screen.queryByRole('button', { name: 'Card' })).toBeNull()
    cleanup()

    render(<PaneCorner tabId="tab_card" what="terminal" />)
    fireEvent.click(screen.getByRole('button', { name: 'Card' }))
    expect(shell.show).toHaveBeenCalledWith('board')
    expect(shell.openCard).toHaveBeenCalledWith('card_1')
  })
})
