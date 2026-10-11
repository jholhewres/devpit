import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { AgentLaunch } from '../gen/bindings'
import { failingWords, heldWords } from './startChecks'

const launches: unknown[][] = []
let answers: AgentLaunch[] = []

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    sessionEnsure: () => ({ tree: { type: 'leaf', id: 'leaf_1' }, focusedId: 'leaf_1' }),
    sessionLaunchAgent: (...args: unknown[]) => (launches.push(args), answers.shift()),
  },
}))
vi.mock('./shellStore', () => ({
  useShellPick: (pick: (shell: unknown) => unknown) =>
    pick({ attach: () => {}, launched: () => {}, agentSessions: {}, active: null, project: null }),
}))
vi.mock('./BlockTerm', () => ({ BlockTerm: () => <div>shell</div> }))
vi.mock('./Split', () => ({ Split: ({ leaf }: { leaf: (id: string) => React.ReactNode }) => leaf('leaf_1') }))
vi.mock('./PaneCorner', () => ({ PaneCorner: () => null }))
vi.mock('./SessionCost', () => ({ SessionCost: () => null }))
vi.mock('./useShortcut', () => ({ useShortcut: () => {} }))
vi.mock('./paneActions', () => ({
  usePaneActions: () => ({ toChat: null, closePane: () => {}, closeLabel: 'Close', armed: false, separate: null }),
}))

const { TerminalPane } = await import('./TerminalPane')

const EXPIRED = { what: "the Claude account Work's sign-in has expired", command: 'claude auth login' }
const started: AgentLaunch = { line: 'claude', missing: [], failing: [] }

beforeEach(() => {
  launches.length = 0
})
afterEach(cleanup)

describe('an agent started from the window', () => {
  it('is held by an expired login, with the command that signs in again, until started anyway', async () => {
    answers = [{ line: null, missing: [EXPIRED], failing: [] }, started]
    render(<TerminalPane tab={{ id: 't1', kind: 'term', launch: 'p-work' }} projectId="p1" />)
    const held = await screen.findByRole('alert')
    expect(held.textContent).toContain("The Claude account Work's sign-in has expired: run claude auth login here.")
    expect(launches).toEqual([['p1', 'leaf_1', 'p-work', false]])

    fireEvent.click(screen.getByRole('button', { name: 'Start anyway' }))
    await vi.waitFor(() => expect(launches[1]).toEqual(['p1', 'leaf_1', 'p-work', true]))
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('goes ahead when a server it needs is down, and says so', async () => {
    answers = [{ ...started, failing: [{ name: 'github', target: 'x', state: 'failed', detail: 'HTTP 400' }] }]
    render(<TerminalPane tab={{ id: 't2', kind: 'term', launch: 'claude' }} projectId="p1" />)
    expect(await screen.findByText(/github is not answering \(HTTP 400\)/)).toBeTruthy()
    expect(screen.queryByRole('alert')).toBeNull()
    expect(launches).toHaveLength(1)
  })
})

describe('what a start says', () => {
  it('names each missing sign-in with its fix, and nothing when every server answers', () => {
    expect(heldWords([EXPIRED])).toBe("Not started: the Claude account Work's sign-in has expired — run `claude auth login`.")
    expect(failingWords([])).toBeNull()
  })
})
