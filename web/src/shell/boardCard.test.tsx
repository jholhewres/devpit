import { cleanup, render, screen } from '@testing-library/react'
import { useState } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Board } from '../gen/bindings'
import { BoardPane } from './BoardPane'

afterEach(cleanup)

const shell = {
  project: { id: 'p1' },
  wantedCard: null as string | null,
  openCard: vi.fn(),
  active: { id: 'board', kind: 'board' } as { id: string; kind: string } | null,
}

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    boardGet: (): Board => ({ projectId: 'p1', columns: [], cards: [], steps: [] }),
    boardArchived: () => ({ cards: [] }),
  },
}))
vi.mock('./window', () => ({ onCarried: () => () => undefined }))
vi.mock('./useShell', () => ({ useShell: () => shell }))
/* The card as the board mounts it: the id it was first given says whether it is a fresh one. */
vi.mock('./CardPane', () => ({
  CardPane: ({ cardId }: { cardId: string }) => {
    const [first] = useState(cardId)
    return <div role="dialog" aria-label="Card" data-first={first} />
  },
}))

const asked = (cardId: string, kind = 'board'): void => {
  shell.wantedCard = cardId
  shell.active = { id: kind, kind }
}

describe('a card open on the board', () => {
  it('is not there while another tab is in front, and is again once the board is', () => {
    asked('card_1')
    const { rerender } = render(<BoardPane />)
    expect(screen.getByRole('dialog', { name: 'Card' })).toBeTruthy()

    shell.wantedCard = null
    shell.active = { id: 'chat_1', kind: 'chat' }
    rerender(<BoardPane />)
    expect(screen.queryByRole('dialog')).toBeNull()

    shell.active = { id: 'board', kind: 'board' }
    rerender(<BoardPane />)
    expect(screen.getByRole('dialog', { name: 'Card' })).toBeTruthy()
  })

  it('is a new one when another card is asked for, not the last one under a new id', () => {
    asked('card_1')
    const { rerender } = render(<BoardPane />)
    asked('card_2')
    rerender(<BoardPane />)
    expect(screen.getByRole('dialog', { name: 'Card' }).getAttribute('data-first')).toBe('card_2')
  })
})
