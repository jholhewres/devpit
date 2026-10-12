import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { ChannelRules, Channels } from '../gen/bindings'

const saved = vi.fn()
const rules: ChannelRules = {
  routes: [
    { event: 'session_waiting', channels: ['telegram', 'devpit_app'], escalateAfter: 10 },
    { event: 'step_failed', channels: [], escalateAfter: null },
  ],
  quiet: null,
  groupSeconds: 60,
}
const channels = (kept: ChannelRules): Channels => ({
  connected: [
    { id: 'telegram', label: 'Telegram, @my_bot' },
    { id: 'devpit_app', label: 'devpit.app (e-mail, push)' },
  ],
  rules: kept,
  escalates: ['session_waiting', 'session_failed', 'session_done', 'draft_ready', 'reminder'],
})

vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    channelsRead: async () => channels(rules),
    channelsRulesSet: async (kept: ChannelRules) => (saved(kept), channels(kept)),
    telegramStatus: async () => ({ linked: true, bot: 'my_bot', code: null, link: null, privacy: 'minimal', orchestrator: null, orchestrators: [] }),
    hubStatus: async () => ({ on: true, titles: false, signedIn: true }),
    urlOpen: async () => ({ path: '' }),
  },
}))

import { ChannelsPane } from './ChannelsPane'

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

describe('moving a notice on in Settings → Channels', () => {
  it('sets the minutes a notice waits before the next channel, per rule', async () => {
    render(<ChannelsPane />)
    const waiting = (await screen.findByRole('combobox', { name: 'A session waits on you: if nobody acts' })) as HTMLSelectElement
    expect(waiting.value).toBe('10')
    fireEvent.change(waiting, { target: { value: '30' } })
    await waitFor(() => expect(saved).toHaveBeenCalled())
    const sent: ChannelRules = saved.mock.calls[0][0]
    expect(sent.routes.find((one) => one.event === 'session_waiting')).toMatchObject({ channels: ['telegram', 'devpit_app'], escalateAfter: 30 })
  })

  it('sends a notice everywhere at once when no delay is picked', async () => {
    render(<ChannelsPane />)
    fireEvent.change(await screen.findByRole('combobox', { name: 'A session waits on you: if nobody acts' }), { target: { value: '' } })
    await waitFor(() => expect(saved).toHaveBeenCalled())
    expect(saved.mock.calls[0][0].routes.find((one: { event: string }) => one.event === 'session_waiting').escalateAfter).toBeNull()
  })

  it('offers no delay where devpit cannot tell the notice was dealt with, and says it needs this computer on', async () => {
    render(<ChannelsPane />)
    await screen.findByRole('combobox', { name: 'A session waits on you: if nobody acts' })
    expect(screen.queryByRole('combobox', { name: 'A step failed: if nobody acts' })).toBeNull()
    expect(screen.getByText(/while it is off or asleep, nothing moves on/)).toBeTruthy()
  })
})
