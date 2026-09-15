import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { CardSession } from '../gen/bindings'
import { CardSessions, READ_AFTER_MS } from './CardSessions'

afterEach(cleanup)

const called = vi.fn()
const shown = vi.fn()
const handlers: Record<string, (payload: unknown) => void> = {}

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    sessionFocus: (...args: unknown[]) => called('sessionFocus', ...args),
    agentProfiles: () => [
      { id: 'prof_mine', reach: 'runnable', driver: 'claude', mine: true },
      { id: 'prof_1', reach: 'runnable', driver: 'claude', mine: false },
    ],
    chatAdopt: (...args: unknown[]) => {
      called('chatAdopt', ...args)
      return 'conv_9'
    },
    runCancel: (...args: unknown[]) => called('runCancel', ...args),
    terminalAttachAgent: (...args: unknown[]) => {
      called('terminalAttachAgent', ...args)
      return { layout: {}, tabId: 'tab_named_by_the_backend', cardId: 'card_1' }
    },
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'p1' }, show: shown }) }))
vi.mock('./window', () => ({
  onCarried: (name: string, then: (payload: unknown) => void) => {
    handlers[name] = then
    return () => undefined
  },
}))

beforeEach(() => {
  called.mockClear()
  shown.mockClear()
})

const session = (over: Partial<CardSession>): CardSession => ({
  kind: 'pane',
  ref: 'leaf_1',
  state: null,
  tabId: null,
  leafId: null,
  runId: null,
  ...over,
})

const rows = (sessions: CardSession[], onChanged = vi.fn()) => {
  render(<CardSessions cardId="card_1" title="Wire the board" sessions={sessions} onChanged={onChanged} />)
  return onChanged
}

const row = (name: string) => within(screen.getByLabelText(name))

describe("a card's Sessions", () => {
  it('goes to the pane a session is in', async () => {
    rows([session({ kind: 'pane', ref: 'leaf_1', state: 'waiting', tabId: 'tab_1', leafId: 'leaf_1' })])
    fireEvent.click(row('Terminal').getByRole('button', { name: 'Go to terminal' }))
    await waitFor(() => expect(called).toHaveBeenCalledWith('sessionFocus', 'p1', 'tab_1', 'leaf_1'))
    expect(shown).toHaveBeenCalledWith('term', { id: 'tab_1', title: 'Wire the board', cardId: 'card_1' })
    expect(row('Terminal').queryByRole('button', { name: 'Open as chat' })).toBeNull()
  })

  it('opens a chat of the card', () => {
    rows([session({ kind: 'chat', ref: 'conv_1', state: 'done' })])
    fireEvent.click(row('Chat').getByRole('button', { name: 'Open chat' }))
    expect(shown).toHaveBeenCalledWith('chat', { id: 'conv_1' })
  })

  it('opens a finished run as a chat, filed under the card', async () => {
    const onChanged = rows([session({ kind: 'run', ref: 's-run', state: 'done', runId: 'run_1' })])
    fireEvent.click(row('Run').getByRole('button', { name: 'Open as chat' }))
    await waitFor(() => expect(shown).toHaveBeenCalledWith('chat', { id: 'conv_9' }))
    expect(called).toHaveBeenCalledWith('chatAdopt', 'p1', 's-run', 'prof_1', null, 'card_1')
    expect(onChanged).toHaveBeenCalled()
  })

  it('stops a run that is working, and will not take it into a chat until then', async () => {
    rows([session({ kind: 'run', ref: 's-run', state: 'working', runId: 'run_1' })])
    expect((row('Run').getByRole('button', { name: 'Open as chat' }) as HTMLButtonElement).disabled).toBe(true)
    fireEvent.click(row('Run').getByRole('button', { name: 'Stop' }))
    await waitFor(() => expect(called).toHaveBeenCalledWith('runCancel', 'card_1', 'run_1'))
  })

  it('attaches a background session in the card terminal, and takes it into a chat only once it is done', async () => {
    rows([session({ kind: 'background', ref: 's-bg', state: 'open' })])
    expect((row('Background session').getByRole('button', { name: 'Open as chat' }) as HTMLButtonElement).disabled).toBe(true)
    fireEvent.click(row('Background session').getByRole('button', { name: 'Attach in terminal' }))
    await waitFor(() =>
      expect(shown).toHaveBeenCalledWith('term', { id: 'tab_named_by_the_backend', title: 'Wire the board', cardId: 'card_1' }),
    )
    expect(called).toHaveBeenCalledWith('terminalAttachAgent', 'p1', 'card_1')
  })

  it('reads the card again once its sessions pause, not once per word', () => {
    vi.useFakeTimers()
    const onChanged = rows([session({ kind: 'run', ref: 's-run', state: 'working', runId: 'run_1' })])
    handlers['card:happening']!({ cardId: 'card_2', activity: 'done', sessions: [] })
    handlers['card:happening']!({ cardId: 'card_1', activity: 'working', sessions: [] })
    handlers['card:happening']!({ cardId: 'card_1', activity: 'done', sessions: [] })
    vi.advanceTimersByTime(READ_AFTER_MS - 1)
    expect(onChanged).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(onChanged).toHaveBeenCalledTimes(1)
    vi.useRealTimers()
  })
})
