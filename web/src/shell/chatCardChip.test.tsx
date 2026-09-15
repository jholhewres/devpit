import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { ChatCardChip } from './ChatCardChip'

afterEach(cleanup)

const shown = vi.fn()
const opened = vi.fn()
vi.mock('./useShell', () => ({ useShell: () => ({ show: shown, openCard: opened }) }))

describe("a chat's card chip", () => {
  it('opens the card while it is on a board', () => {
    render(<ChatCardChip card={{ id: 'card_1', title: 'Wire the board', onBoard: true }} />)
    fireEvent.click(screen.getByRole('button', { name: 'Wire the board' }))
    expect(shown).toHaveBeenCalledWith('board')
    expect(opened).toHaveBeenCalledWith('card_1')
  })

  it('is only a name once the card is off the board', () => {
    render(<ChatCardChip card={{ id: 'card_1', title: 'Wire the board', onBoard: false }} />)
    expect(screen.getByText('Wire the board')).toBeTruthy()
    expect(screen.queryByRole('button')).toBeNull()
  })

  it('draws nothing for a conversation about no card', () => {
    const { container } = render(<ChatCardChip card={null} />)
    expect(container.innerHTML).toBe('')
  })
})
