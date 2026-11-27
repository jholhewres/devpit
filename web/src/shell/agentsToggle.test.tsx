import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { setLocale } from '../i18n'

const set = vi.fn()
let kept = false
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    agentsMode: async () => ({ on: kept }),
    agentsModeSet: async (projectId: string, on: boolean) => {
      set(projectId, on)
      kept = on
      return { on }
    },
  },
}))

const { AgentsToggle } = await import('./AgentsToggle')

afterEach(() => {
  cleanup()
  set.mockClear()
  kept = false
  setLocale('en')
})

describe('Agents mode in the orchestrator chat', () => {
  it('is off by default, and a click turns it on and keeps it', async () => {
    render(<AgentsToggle projectId="orch" />)
    const toggle = screen.getByRole('button', { name: 'Agents mode' })
    expect(toggle.getAttribute('aria-pressed')).toBe('false')
    fireEvent.click(toggle)
    await waitFor(() => expect(toggle.getAttribute('aria-pressed')).toBe('true'))
    expect(set).toHaveBeenCalledWith('orch', true)
    cleanup()
    // Opened again, it reads what was kept.
    render(<AgentsToggle projectId="orch" />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Agents mode' }).getAttribute('aria-pressed')).toBe('true'))
  })

  it('turns off again on the next click', async () => {
    kept = true
    render(<AgentsToggle projectId="orch" />)
    const toggle = screen.getByRole('button', { name: 'Agents mode' })
    await waitFor(() => expect(toggle.getAttribute('aria-pressed')).toBe('true'))
    fireEvent.click(toggle)
    await waitFor(() => expect(toggle.getAttribute('aria-pressed')).toBe('false'))
    expect(set).toHaveBeenCalledWith('orch', false)
  })

  it('says what it does in Portuguese', async () => {
    setLocale('pt-BR')
    render(<AgentsToggle projectId="orch" />)
    const toggle = screen.getByRole('button', { name: 'Modo Agentes' })
    expect(toggle.getAttribute('title')).toContain('Desligado')
  })
})
