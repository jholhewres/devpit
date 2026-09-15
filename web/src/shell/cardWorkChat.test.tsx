import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { CardWork } from './CardWork'

afterEach(cleanup)

const chatted = vi.fn()
const shown = vi.fn()

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    cardTerminal: () => ({ layout: {}, tabId: 'tab_named_by_the_backend', cardId: 'card_1' }),
    agentProfiles: () => [
      { id: 'prof_1', path: '/usr/bin/claude' },
      { id: 'prof_gone', path: null },
    ],
    cardDetail: () => ({
      card: { title: 'Wire the board', body: 'All of it.\n' },
      pinned: [{ path: '/w/notes.md', label: 'notes.md' }],
    }),
    cardChat: (project: string, card: string, profile: string | null) => {
      chatted(project, card, profile)
      return 'conv_1'
    },
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'p1' }, show: shown }) }))
vi.mock('./useKnownAgents', () => ({ offered: () => [], useKnownAgents: () => [] }))
vi.mock('./useOpeners', () => ({ useOpeners: () => [] }))

const work = () => render(<CardWork cardId="card_1" title="Wire the board" worktree={null} runs={[]} onChanged={vi.fn()} />)

describe('a chat about the card', () => {
  it('opens in a chat tab with the card in its composer, under the one account installed', async () => {
    work()
    fireEvent.click(screen.getByRole('button', { name: 'Chat about this card' }))
    await waitFor(() =>
      expect(shown).toHaveBeenCalledWith('chat', {
        id: 'conv_1',
        draft: '# Wire the board\n\nAll of it.\n\nPinned files:\n- /w/notes.md',
      }),
    )
    expect(chatted).toHaveBeenCalledWith('p1', 'card_1', 'prof_1')
  })
})

describe("a card's terminal", () => {
  it('opens in a tab named after the card, not its branch', async () => {
    work()
    fireEvent.click(screen.getByRole('button', { name: 'Open a terminal' }))
    await waitFor(() =>
      expect(shown).toHaveBeenCalledWith('term', { id: 'tab_named_by_the_backend', title: 'Wire the board', cardId: 'card_1' }),
    )
  })
})
