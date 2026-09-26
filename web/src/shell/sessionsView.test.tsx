import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { LiveSession } from '../gen/bindings'
import { since, SessionsView } from './SessionsView'

const replied = vi.fn()
const live = (name: string, projectName: string, extra: Partial<LiveSession> = {}): LiveSession =>
  ({ name, status: 'idle', kind: 'interactive', cwd: '/w', projectId: 'p', projectName, cardId: null, since: null, inDevpit: true, waiting: null, pane: null, ...extra }) as LiveSession

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    orchestratorSessions: () => ({
      sessions: [
        live('api-b', 'api'),
        live('web-a', 'web', { status: 'busy', projectId: 'w' }),
        live('api-a', 'api', { waiting: { question: 'Deploy now?', options: [], cursor: 0 }, pane: { projectId: 'p', paneId: 'leaf_1' } }),
      ],
    }),
    orchestratorAgents: () => [
      { name: 'api-a', lastAt: '2026-09-25T18:13:45Z', events: [{ kind: 'sent', at: '2026-09-25T18:11:55Z', summary: 'Deploy the fix', text: 'Deploy it to dev.' }] },
      { name: 'old-one', lastAt: '2026-09-20T10:00:00Z', events: [{ kind: 'heard', at: '2026-09-20T10:00:00Z', summary: null, text: 'Done.' }] },
    ],
    orchestratorReply: (...args: unknown[]) => (replied(...args), null),
    orchestratorLinks: () => ['p'],
    orchestratorStop: () => 'stopped',
  },
}))
vi.mock('./useShell', () => ({
  useShell: () => ({ project: { id: 'orch', orchestrator: 'claude' }, projects: [{ id: 'p', name: 'api', rootPath: '/w/api' }], setProject: () => {}, show: () => {}, openCard: () => {}, openPane: () => {} }),
}))

afterEach(cleanup)

describe("an orchestrator's sessions", () => {
  it('group by project, the one waiting on the person first, with what was last asked of it', async () => {
    render(<SessionsView shown />)
    const names = (await screen.findAllByText(/^(api|web)-[ab]$/)).map((one) => one.textContent)
    expect(names).toEqual(['api-a', 'api-b', 'web-a'])
    expect(screen.getByText('1 waiting')).toBeTruthy()
    expect(await screen.findByText(/Deploy the fix/)).toBeTruthy()
    expect(screen.getByLabelText("Open api-a's terminal here")).toBeTruthy()
    expect(screen.queryByLabelText("Open api-b's terminal here")).toBeNull()
  })

  it('opens to the question, a reply typed as the person, and the history', async () => {
    render(<SessionsView shown />)
    fireEvent.click(await screen.findByText('api-a'))
    expect(screen.getByText('Deploy now?')).toBeTruthy()
    expect(screen.getByText('Deploy it to dev.')).toBeTruthy()
    const input = screen.getByLabelText('Reply to api-a')
    fireEvent.change(input, { target: { value: 'yes, go' } })
    fireEvent.submit(input.closest('form')!)
    expect(replied).toHaveBeenCalledWith('claude', 'api-a', 'yes, go')
  })

  it('keeps the ones no longer running below, with their history', async () => {
    render(<SessionsView shown />)
    expect(await screen.findByText('Earlier · not running')).toBeTruthy()
    fireEvent.click(screen.getByText('old-one'))
    expect(screen.getByText('Done.')).toBeTruthy()
  })

  it('says how long in words', () => {
    const now = Date.UTC(2026, 8, 25, 12)
    expect(since(null, now)).toBeNull()
    expect(since(now - 20_000, now)).toBe('just now')
    expect(since(now - 12 * 60_000, now)).toBe('12 min')
    expect(since(now - 3 * 3_600_000, now)).toBe('3 h')
  })
})
