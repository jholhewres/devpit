import { render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import type { SessionCost as Cost } from '../gen/bindings'
import { SessionCost } from './SessionCost'
import { costDetail, costWords, tokens } from './sessionCost'

const COST: Cost = {
  costUsd: 1.234,
  lastTurnUsd: 0.2,
  sinceSeenUsd: 0.5,
  tokens: { input: 1200, output: 3400, cacheRead: 2_500_000, cacheWrite: 0 },
  model: 'claude-sonnet-4-5',
  unpricedTokens: 0,
}

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null }),
  commands: { sessionCost: async () => COST },
}))

describe('what a session has spent', () => {
  it('reads as dollars or tokens', () => {
    expect(costWords(COST, 'usd')).toBe('$1.23')
    expect(costWords(COST, 'tokens')).toBe('2.5M tok')
    expect(tokens(3400)).toBe('3k')
  })

  it('says it is the API-equivalent, the last turn, and what was spent here', () => {
    const said = costDetail(COST)
    expect(said).toContain('API-equivalent cost: $1.23')
    expect(said).toContain('Last turn: $0.20')
    expect(said).toContain('Since it was opened here: $0.50')
    expect(said).toContain('Model: claude-sonnet-4-5')
  })

  it('is drawn for a session, and not for none', async () => {
    const { container, rerender } = render(<SessionCost sessionId="s1" />)
    await waitFor(() => expect(screen.getByText('$1.23')).toBeTruthy())
    rerender(<SessionCost sessionId={null} />)
    await waitFor(() => expect(container.querySelector('.scost')).toBeNull())
  })
})
