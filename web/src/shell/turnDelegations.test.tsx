import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Part } from '../gen/bindings'
import { AppsScopeContext } from './mcpApps'
import { stateOf } from './liveStatus'
import { TurnDelegations } from './TurnDelegations'

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    orchestratorSessions: () => ({
      sessions: [{ name: 'gatorclaw-82', status: 'busy', kind: 'interactive', cwd: '/w', projectId: 'p', projectName: 'gatorclaw', cardId: null, since: null, inDevpit: true, waiting: null }],
    }),
  },
}))

vi.mock('./useShell', () => ({ useShell: () => ({ setProject: () => {}, openPane: () => {} }) }))

afterEach(cleanup)

const sent: Part = {
  kind: 'tool_call',
  id: 'toolu_1',
  name: 'SendMessage',
  input: JSON.stringify({ to: 'gatorclaw-82', summary: 'Build and deploy dev', message: 'Update master, build, deploy.' }),
  state: 'ok',
}

describe('work handed to another session', () => {
  it('is a card saying to whom, what, and how that session stands now', async () => {
    render(
      <AppsScopeContext.Provider value={{ profileId: 'claude', cwd: '/w' }}>
        <TurnDelegations parts={[sent]} />
      </AppsScopeContext.Provider>,
    )
    expect(screen.getByText('→ gatorclaw-82')).toBeTruthy()
    expect(screen.getByText('Build and deploy dev')).toBeTruthy()
    expect(await screen.findByText('working')).toBeTruthy()
    fireEvent.click(screen.getByText('Build and deploy dev'))
    expect(screen.getByText('Update master, build, deploy.')).toBeTruthy()
  })

  /* SendMessage answers a send that did not go with `success: false`, not with an error. */
  const unreachable = (callId: string): Part => ({
    kind: 'tool_result',
    call_id: callId,
    output: JSON.stringify({ success: false, message: "No agent named 'gatorclaw-82' is reachable.\nUse ListAgents to see everyone you can message." }),
    is_error: false,
  })
  const sentAgain: Part = { ...sent, id: 'toolu_2' } as Part
  const delivered: Part = { kind: 'tool_result', call_id: 'toolu_2', output: JSON.stringify({ success: true, message: 'sent' }), is_error: false }

  it('shows a send that did not go as not sent, with why', () => {
    render(
      <AppsScopeContext.Provider value={{ profileId: 'claude', cwd: '/w' }}>
        <TurnDelegations parts={[sent, unreachable('toolu_1')]} />
      </AppsScopeContext.Provider>,
    )
    expect(screen.getByText('not sent')).toBeTruthy()
    expect(screen.queryByText('working')).toBeNull()
    expect(screen.getByText("No agent named 'gatorclaw-82' is reachable.")).toBeTruthy()
  })

  it('keeps one card for a send tried again, the one that went', async () => {
    render(
      <AppsScopeContext.Provider value={{ profileId: 'claude', cwd: '/w' }}>
        <TurnDelegations parts={[sent, unreachable('toolu_1'), sentAgain, delivered]} />
      </AppsScopeContext.Provider>,
    )
    expect(screen.getAllByText('→ gatorclaw-82')).toHaveLength(1)
    expect(screen.queryByText('not sent')).toBeNull()
    expect(await screen.findByText('working')).toBeTruthy()
  })

  it('is nothing for a turn that sent nothing', () => {
    const { container } = render(<TurnDelegations parts={[]} />)
    expect(container.textContent).toBe('')
  })

  it('reads a session as waiting before anything else', () => {
    const one = { name: 'a', status: 'busy', waiting: { question: 'Go?', options: [], cursor: 0 } }
    expect(stateOf([one as never], 'a')).toBe('waiting')
    expect(stateOf([], 'a')).toBe('gone')
  })
})
