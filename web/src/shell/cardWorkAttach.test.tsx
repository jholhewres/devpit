import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { CardSession } from '../gen/bindings'
import { CardWork } from './CardWork'

afterEach(cleanup)

const attached = vi.fn()
const chatted = vi.fn()
const shown = vi.fn()

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    agentProfiles: () => [
      { id: 'prof_1', path: '/usr/bin/claude' },
      { id: 'prof_gone', path: null },
    ],
    cardChat: (project: string, card: string, profile: string | null) => {
      chatted(project, card, profile)
      return 'conv_1'
    },
    terminalAttachAgent: (project: string, card: string) => {
      attached(project, card)
      return {
        layout: { projectId: 'p1', focusedId: 'leaf_1', tree: { type: 'leaf', id: 'leaf_1' } },
        tabId: 'tab_named_by_the_backend',
        cardId: 'card_1',
      }
    },
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'p1' }, show: shown }) }))
vi.mock('./useKnownAgents', () => ({ offered: () => [], useKnownAgents: () => [] }))
vi.mock('./useOpeners', () => ({ useOpeners: () => [] }))

const background: CardSession = { kind: 'background', ref: 's-bg', state: 'waiting', tabId: null, leafId: null }

const work = (sessions: readonly CardSession[]) =>
  render(<CardWork cardId="card_1" worktree={null} runs={[]} sessions={sessions} onChanged={vi.fn()} />)

describe("a card's background session", () => {
  it('is attached in the card terminal the backend names', async () => {
    work([background])
    expect(screen.getByText('waiting')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Attach in terminal' }))
    await waitFor(() => expect(shown).toHaveBeenCalled())
    expect(attached).toHaveBeenCalledWith('p1', 'card_1')
    expect(shown).toHaveBeenCalledWith('term', { id: 'tab_named_by_the_backend', title: 'Card', cardId: 'card_1' })
  })

  it('has no row when the card has no background session', () => {
    work([{ kind: 'pane', ref: 'leaf_1', state: null, tabId: 'tab_1', leafId: 'leaf_1' }])
    expect(screen.queryByRole('button', { name: 'Attach in terminal' })).toBeNull()
  })
})

describe('a chat about the card', () => {
  it('opens in a chat tab, under the one account installed', async () => {
    work([])
    fireEvent.click(screen.getByRole('button', { name: 'Chat about this card' }))
    await waitFor(() => expect(shown).toHaveBeenCalledWith('chat', { id: 'conv_1' }))
    expect(chatted).toHaveBeenCalledWith('p1', 'card_1', 'prof_1')
  })
})
