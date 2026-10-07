import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Deciding } from '../gen/bindings'

const kept = vi.fn()
const sent = vi.fn()
let settings: Deciding

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    decisionsRead: () => settings,
    decisionsSet: (provider: string, model: string, url: string, dailyCapUsd: number) => ({ ...settings, provider, model, url, dailyCapUsd }),
    decisionsKeySet: (key: string | null) => (kept(key), null),
    decisionsProjectSet: (projectId: string, send: boolean) => (sent(projectId, send), { ...settings, optedOut: send ? [] : [projectId] }),
    decisionsTest: () => ({ probability: 0.97, latencyMs: 640, costUsd: 0.0004, note: null }),
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'prj_api', name: 'api' } }) }))

const { DecisionsSettings } = await import('./DecisionsSettings')

const off: Deciding = {
  provider: 'openrouter',
  model: '',
  url: '',
  dailyCapUsd: 1,
  keySet: false,
  spentTodayUsd: 0,
  decidedToday: 0,
  optedOut: [],
}

afterEach(cleanup)

describe('Decisions in Settings', () => {
  it('is off until a key is kept, and keeps the key it is given', async () => {
    settings = off
    render(<DecisionsSettings />)
    expect(await screen.findByText(/Off until a key is kept/)).toBeTruthy()
    expect((screen.getByRole('button', { name: 'Test' }) as HTMLButtonElement).disabled).toBe(true)
    fireEvent.change(screen.getByPlaceholderText('Paste your OpenRouter key'), { target: { value: 'sk-or-1' } })
    fireEvent.click(screen.getByRole('button', { name: 'Keep' }))
    expect(await screen.findByRole('button', { name: 'Forget' })).toBeTruthy()
    expect(kept).toHaveBeenCalledWith('sk-or-1')
  })

  it('tests the key and says what the answer took and cost', async () => {
    settings = { ...off, keySet: true }
    render(<DecisionsSettings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Test' }))
    expect(await screen.findByText('Answered in 640 ms for $0.0004 — p 0.97.')).toBeTruthy()
  })

  it("keeps the current project's state from being sent", async () => {
    settings = { ...off, keySet: true }
    render(<DecisionsSettings />)
    const toggle = await screen.findByRole('switch', { name: "Send api's state" })
    expect(toggle.getAttribute('aria-checked')).toBe('true')
    fireEvent.click(toggle)
    expect(sent).toHaveBeenCalledWith('prj_api', false)
    expect((await screen.findByRole('switch', { name: "Send api's state" })).getAttribute('aria-checked')).toBe('false')
  })
})
