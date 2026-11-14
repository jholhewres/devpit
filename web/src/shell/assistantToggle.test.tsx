import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

const set = vi.fn()
let kept = false
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    assistantMode: async () => ({ on: kept }),
    assistantModeSet: async (projectId: string, on: boolean) => {
      set(projectId, on)
      kept = on
      return { on }
    },
  },
}))

const { AssistantToggle } = await import('./AssistantToggle')

afterEach(() => {
  cleanup()
  set.mockClear()
  kept = false
})

describe('Assistant mode in the orchestrator chat', () => {
  it('is off by default, and a click turns it on and keeps it', async () => {
    render(<AssistantToggle projectId="orch" />)
    const toggle = screen.getByRole('button', { name: 'Assistant mode' })
    expect(toggle.getAttribute('aria-pressed')).toBe('false')
    fireEvent.click(toggle)
    await waitFor(() => expect(toggle.getAttribute('aria-pressed')).toBe('true'))
    expect(set).toHaveBeenCalledWith('orch', true)
    cleanup()
    // Opened again, it reads what was kept.
    render(<AssistantToggle projectId="orch" />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Assistant mode' }).getAttribute('aria-pressed')).toBe('true'))
  })

  it('turns off again on the next click', async () => {
    kept = true
    render(<AssistantToggle projectId="orch" />)
    const toggle = screen.getByRole('button', { name: 'Assistant mode' })
    await waitFor(() => expect(toggle.getAttribute('aria-pressed')).toBe('true'))
    fireEvent.click(toggle)
    await waitFor(() => expect(toggle.getAttribute('aria-pressed')).toBe('false'))
    expect(set).toHaveBeenCalledWith('orch', false)
  })
})
