import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { LiveSession, Part } from '../gen/bindings'
import { draftsIn, standing, TurnDrafts } from './TurnDrafts'

const replied = vi.fn(async (..._args: unknown[]) => null as unknown)
let sessions: LiveSession[] = []
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    orchestratorReply: (...args: unknown[]) => replied(...args),
    orchestratorDraftDrop: async () => undefined,
  },
}))
vi.mock('./liveStatus', () => ({ useLiveSessions: () => sessions, refreshSessions: () => undefined }))
vi.mock('./shellStore', () => ({ useShellPick: (pick: (shell: unknown) => unknown) => pick({ project: { orchestrator: 'claude' } }) }))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

const live = (draft: string | null, extra: Partial<LiveSession> = {}): LiveSession =>
  ({ name: 'api-a', status: 'idle', waiting: null, pane: { projectId: 'p', paneId: 'leaf_1' }, draft, ...extra }) as LiveSession

const drafted: Part = {
  kind: 'tool_call',
  id: 'toolu_1',
  name: 'mcp__devpit__devpit_draft_reply',
  input: JSON.stringify({ name: 'api-a', text: 'yes, go on' }),
  state: 'ok',
  parent: null,
}

const shown = () =>
  render(<TurnDrafts parts={[drafted]} />)

describe('a draft in the orchestrator chat', () => {
  it('is read from the call that drafted it', () => {
    expect(draftsIn([drafted])).toEqual([{ id: 'toolu_1', to: 'api-a', text: 'yes, go on' }])
  })

  it('is pending only while it is the session draft', () => {
    const draft = draftsIn([drafted])[0]!
    expect(standing(draft, live('yes, go on'), null)).toBe('pending')
    expect(standing(draft, live('something newer'), null)).toBe('gone')
    expect(standing(draft, null, null)).toBe('gone')
  })

  it('sends what the person edited, to that session, on their click', async () => {
    sessions = [live('yes, go on')]
    shown()
    fireEvent.click(screen.getByText('Edit'))
    fireEvent.change(screen.getByLabelText('Reply to api-a, as you'), { target: { value: 'yes, but skip the migration' } })
    fireEvent.click(screen.getByText('Send'))
    await waitFor(() => expect(replied).toHaveBeenCalledWith('claude', 'api-a', 'yes, but skip the migration', 'the chat'))
    expect(await screen.findByText(/Sent as you at/)).toBeTruthy()
  })

  it('offers nothing to send once it is no longer pending', () => {
    sessions = [live(null)]
    shown()
    expect(screen.queryByText('Send')).toBeNull()
    expect(screen.getByText(/No longer pending/)).toBeTruthy()
  })
})
