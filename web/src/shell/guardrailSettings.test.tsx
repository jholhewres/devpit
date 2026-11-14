import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Guardrail, GuardrailMode, Guardrails } from '../gen/bindings'

const saved = vi.fn()
const untouched: Guardrails = {
  rows: [
    { guardrail: 'stop', mode: 'follow' },
    { guardrail: 'forceStop', mode: 'always' },
    { guardrail: 'sendDraft', mode: 'follow' },
  ],
}
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    orchestratorGuardrails: async (): Promise<Guardrails> => untouched,
    orchestratorGuardrailSet: async (projectId: string, guardrail: Guardrail, mode: GuardrailMode): Promise<Guardrails> => (
      saved(projectId, guardrail, mode),
      { rows: untouched.rows.map((row) => (row.guardrail === guardrail ? { guardrail, mode } : row)) }
    ),
  },
}))
vi.mock('./useShell', () => ({
  useShell: () => ({
    projects: [
      { id: 'prj_api', name: 'api', orchestrator: null },
      { id: 'prj_orch', name: 'Space', orchestrator: 'prof_a' },
    ],
  }),
}))

const { GuardrailSettings } = await import('./GuardrailSettings')

afterEach(() => {
  cleanup()
  saved.mockClear()
})

describe('Settings → Orchestrator → Guardrails', () => {
  it('lists each guardrail with its three modes, as they are untouched', async () => {
    render(<GuardrailSettings />)
    const stop = (await screen.findByLabelText('Stop a session')) as HTMLSelectElement
    expect(stop.value).toBe('follow')
    expect([...stop.options].map((one) => one.text)).toEqual(['Follow chat permission', 'Always ask', 'Never ask'])
    expect((screen.getByLabelText('Force stop, or a stop that loses work') as HTMLSelectElement).value).toBe('always')
    expect((screen.getByLabelText('Send a drafted reply as you') as HTMLSelectElement).value).toBe('follow')
  })

  it('keeps a mode the person picks, for that orchestrator', async () => {
    render(<GuardrailSettings />)
    fireEvent.change(await screen.findByLabelText('Send a drafted reply as you'), { target: { value: 'never' } })
    await waitFor(() => expect(saved).toHaveBeenLastCalledWith('prj_orch', 'sendDraft', 'never'))
    await waitFor(() => expect((screen.getByLabelText('Send a drafted reply as you') as HTMLSelectElement).value).toBe('never'))
  })
})
