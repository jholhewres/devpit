import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

let known = [{ id: 'codex', label: 'Codex', launch: 'codex', installed: true }]
vi.mock('./useKnownAgents', () => ({ useKnownAgents: () => known }))

const called = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    devflowPrefs: async () => ({ enabled: true, preset: 'recommended' }),
    devflowPrefsSet: async (enabled: boolean | null, preset: string | null) => {
      called('set', enabled, preset)
      return { enabled: enabled ?? true, preset: preset ?? 'recommended' }
    },
    devflowCodexInstall: async () => {
      called('codex')
      return { folder: '/h/devflow-plugin/codex', said: 'Installed.' }
    },
  },
}))

const { DevflowSettings } = await import('./DevflowSettings')

afterEach(() => {
  cleanup()
  called.mockReset()
  known = [{ id: 'codex', label: 'Codex', launch: 'codex', installed: true }]
})

describe('Devflow in Settings', () => {
  it('switches Devflow for every project, and sets where a new setup starts', async () => {
    render(<DevflowSettings />)
    const toggle = await screen.findByRole('switch', { name: 'Devflow' })
    await waitFor(() => expect(toggle.hasAttribute('disabled')).toBe(false))
    fireEvent.click(toggle)
    await waitFor(() => expect(called).toHaveBeenCalledWith('set', false, null))
    fireEvent.change(screen.getByLabelText('A new setup starts from'), { target: { value: 'custom' } })
    await waitFor(() => expect(called).toHaveBeenCalledWith('set', null, 'custom'))
  })

  it("installs for Codex only after a yes to what it will run, and only where Codex is", async () => {
    render(<DevflowSettings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Install for Codex…' }))
    expect(called).not.toHaveBeenCalledWith('codex')
    expect(screen.getByText(/codex plugin add devflow@devpit/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Install' }))
    expect(await screen.findByText('Installed.')).toBeTruthy()
    cleanup()
    known = [{ id: 'codex', label: 'Codex', launch: 'codex', installed: false }]
    render(<DevflowSettings />)
    await screen.findByRole('switch', { name: 'Devflow' })
    expect(screen.queryByRole('button', { name: 'Install for Codex…' })).toBeNull()
  })
})
