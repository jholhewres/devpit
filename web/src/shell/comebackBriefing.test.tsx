import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

const asked = vi.fn()
const zoned = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    schedulesList: async (projectId: string, zone: string, offset: number) => (zoned(projectId, zone, offset), { schedules: [], proposed: [] }),
    assistantBriefing: async (projectId: string, since: number) => {
      asked(projectId, since)
      return {
        since,
        lines: ['Sessions: 2 running, 1 waiting on you, 3 ended.', 'Merged: api #12 Fix the login.', 'Decisions waiting: 1 schedule proposal.'],
      }
    },
  },
}))

const { AWAY_MS, ComebackBriefing } = await import('./ComebackBriefing')

afterEach(() => {
  cleanup()
  asked.mockClear()
  zoned.mockClear()
  vi.useRealTimers()
})

const away = async (ms: number): Promise<void> => {
  await act(async () => {
    window.dispatchEvent(new Event('blur'))
    vi.setSystemTime(Date.now() + ms)
    window.dispatchEvent(new Event('focus'))
  })
}

describe('the comeback briefing in the orchestrator chat', () => {
  it('summarises the sessions, the merged PRs and the decisions waiting after a long absence', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-11-13T09:00:00Z'))
    render(<ComebackBriefing projectId="orch" />)
    const left = Math.floor(Date.now() / 1000)
    await away(AWAY_MS + 1000)
    const said = await screen.findByRole('status', { name: 'While you were away' })
    expect(asked).toHaveBeenCalledWith('orch', left)
    expect(said.textContent).toContain('Sessions: 2 running')
    expect(said.textContent).toContain('Merged: api #12')
    expect(said.textContent).toContain('Decisions waiting')
    // The window's zone went to the routines as it opened and as it came back.
    expect(zoned).toHaveBeenCalledTimes(2)
    expect(zoned).toHaveBeenLastCalledWith('orch', expect.any(String), -new Date().getTimezoneOffset())
  })

  it('says nothing after a short absence', async () => {
    // A day later: a return is told once a second at most, whatever the test before did.
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-11-14T09:00:00Z'))
    render(<ComebackBriefing projectId="orch" />)
    await away(60 * 1000)
    await waitFor(() => expect(asked).not.toHaveBeenCalled())
    expect(screen.queryByRole('status')).toBeNull()
  })
})
