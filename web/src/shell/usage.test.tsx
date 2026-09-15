import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { PlanLimits, SpendHistory } from '../gen/bindings'
import { Usage } from './Usage'

afterEach(cleanup)

const asked = vi.fn()
const shown = vi.fn()
const opened = vi.fn()
let held: ((days: number) => Promise<SpendHistory>) | null = null

const history = (): SpendHistory => ({
  days: 30,
  installations: [
    { directory: '/h/.claude', label: '.claude', billed: true },
    { directory: '/h/.claude-glm', label: 'glm', billed: false },
  ],
  costUsd: 12.345,
  tokens: { input: 1_000, output: 2_000, cacheRead: 6_000, cacheWrite: 1_000 },
  sessions: 3,
  turns: 40,
  activeDays: 2,
  cacheReuse: 0.75,
  daily: [
    { day: '2026-09-14', costUsd: 2, tokens: { input: 1, output: 1, cacheRead: 0, cacheWrite: 0 }, byModel: [] },
    { day: '2026-09-15', costUsd: 10.345, tokens: { input: 1, output: 1, cacheRead: 0, cacheWrite: 0 }, byModel: [] },
  ],
  models: [{ name: 'claude-opus-5', costUsd: 12.345, tokens: { input: 1, output: 1, cacheRead: 0, cacheWrite: 0 }, sessions: 3, lastActive: 1 }],
  projects: [{ name: 'devpit', costUsd: 12.345, tokens: { input: 1, output: 1, cacheRead: 0, cacheWrite: 0 }, sessions: 3, lastActive: 1 }],
  recent: [
    {
      sessionId: 's-card',
      project: 'devpit',
      model: 'claude-opus-5',
      turns: 12,
      tokens: { input: 1, output: 1, cacheRead: 0, cacheWrite: 0 },
      costUsd: 4,
      lastActive: Date.now() / 1000 - 3600,
      installation: '.claude',
      card: { id: 'card_1', title: 'Wire the board' },
    },
  ],
  estimated: false,
  unpricedTokens: 500,
  files: 9,
  records: 40,
  scanMs: 12,
})

const limits = (): PlanLimits => ({
  installation: '/h/.claude',
  plan: 'Max (5x)',
  windows: [
    { label: '5-hour', percent: 42, resetsAt: Date.now() / 1000 + 2 * 3600 + 20 * 60 },
    { label: 'Weekly', percent: 85, resetsAt: null },
  ],
  readAt: 1,
  problem: null,
})

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    spendHistory: (project: string | null, installation: string | null, days: number) => {
      asked('history', project, installation, days)
      return held ? held(days) : history()
    },
    planLimits: (installation: string | null) => {
      asked('limits', installation)
      return limits()
    },
    usageRead: () => ({ usd: 1.5, runs: 1, cards: [['Ship it', 1.5]] }),
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'p1', name: 'devpit' }, show: shown, openCard: opened }) }))

beforeEach(() => {
  held = null
  asked.mockClear()
  shown.mockClear()
  opened.mockClear()
})

describe('the usage screen', () => {
  it('shows the plan windows with their reset, the totals and the sessions', async () => {
    render(<Usage />)
    expect(await screen.findAllByText('$12.35')).toHaveLength(3)
    const plan = screen.getByRole('region', { name: 'Plan limits' })
    expect(within(plan).getByText('Plan · Max (5x)')).toBeTruthy()
    expect(within(plan).getByText('42%')).toBeTruthy()
    expect(within(plan).getByText('resets in 2h 20m')).toBeTruthy()
    expect(screen.getByText('75%')).toBeTruthy()
    expect(screen.getByText(/500 tokens from models with no price/)).toBeTruthy()
    expect(screen.getByText('Ship it')).toBeTruthy()
  })

  it('reads again for the range, the installation and the project chosen', async () => {
    render(<Usage />)
    await screen.findAllByText('$12.35')
    expect(asked).toHaveBeenCalledWith('history', null, null, 30)
    fireEvent.click(screen.getByRole('tab', { name: '7d' }))
    await waitFor(() => expect(asked).toHaveBeenCalledWith('history', null, null, 7))
    fireEvent.change(screen.getByRole('combobox', { name: 'Installation' }), { target: { value: '/h/.claude-glm' } })
    await waitFor(() => expect(asked).toHaveBeenCalledWith('limits', '/h/.claude-glm'))
    fireEvent.click(screen.getByRole('tab', { name: 'devpit' }))
    await waitFor(() => expect(asked).toHaveBeenCalledWith('history', 'p1', '/h/.claude-glm', 7))
    expect(screen.getByRole('option', { name: 'glm (not billed by Anthropic)' })).toBeTruthy()
  })

  it('opens the card a session belongs to', async () => {
    render(<Usage />)
    fireEvent.click(await screen.findByRole('button', { name: 'Wire the board' }))
    expect(shown).toHaveBeenCalledWith('board')
    expect(opened).toHaveBeenCalledWith('card_1')
  })

  it('draws the range asked for last, even when an earlier range answers after it', async () => {
    const answers = new Map<number, (value: SpendHistory) => void>()
    held = (days) => new Promise((resolve) => answers.set(days, resolve))
    render(<Usage />)
    await waitFor(() => expect(answers.has(30)).toBe(true))
    fireEvent.click(screen.getByRole('tab', { name: '7d' }))
    await waitFor(() => expect(answers.has(7)).toBe(true))
    answers.get(7)!({ ...history(), days: 7 })
    expect(await screen.findByText('Last 7 days')).toBeTruthy()
    answers.get(30)!({ ...history(), days: 30 })
    await new Promise((resolve) => setTimeout(resolve, 20))
    expect(screen.queryByText('Last 30 days')).toBeNull()
    expect(screen.getByText('Last 7 days')).toBeTruthy()
  })
})
