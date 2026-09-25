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
