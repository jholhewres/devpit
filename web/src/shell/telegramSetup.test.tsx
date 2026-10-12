import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { TelegramStatus } from '../gen/bindings'

const unlinked: TelegramStatus = { linked: false, bot: null, code: null, link: null, privacy: 'minimal', orchestrator: null, orchestrators: [] }
let status: TelegramStatus = unlinked
const linked = vi.fn()
const privacySet = vi.fn()
const orchestratorSet = vi.fn()
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: {
    telegramStatus: () => status,
    telegramLink: (token: string) => (linked(token), { ...unlinked, bot: 'my_bot', code: 'A1B2C3', link: 'https://t.me/my_bot?start=A1B2C3' }),
    telegramUnlink: () => unlinked,
    telegramPrivacySet: (privacy: string) => (privacySet(privacy), { ...status, privacy }),
    telegramOrchestratorSet: (project: string | null) => (orchestratorSet(project), { ...status, orchestrator: project }),
    urlOpen: () => ({ path: '' }),
  },
}))

import { TelegramSetup } from './TelegramSetup'

afterEach(cleanup)

describe('linking a Telegram bot', () => {
  it('takes the token from a password field and shows the code to send', async () => {
    render(<TelegramSetup onLinked={vi.fn()} />)
    const field = await screen.findByPlaceholderText('123456:ABC…')
    expect((field as HTMLInputElement).type).toBe('password')
    fireEvent.change(field, { target: { value: '123:abc' } })
    fireEvent.click(screen.getByText('Link'))
    await waitFor(() => expect(linked).toHaveBeenCalledWith('123:abc'))
    expect(await screen.findByText('/start A1B2C3')).toBeTruthy()
  })

  it('offers to unlink once a chat is linked', async () => {
    status = { ...unlinked, linked: true, bot: 'my_bot' }
    render(<TelegramSetup onLinked={vi.fn()} />)
    expect(await screen.findByText('Telegram, @my_bot')).toBeTruthy()
    expect(screen.getByText('Unlink')).toBeTruthy()
  })

  it('sets how much the chat is told and which orchestrator it talks to', async () => {
    status = {
      ...unlinked,
      linked: true,
      bot: 'my_bot',
      orchestrators: [
        { project: 'orch-work', name: 'work' },
        { project: 'orch-home', name: 'home' },
      ],
    }
    render(<TelegramSetup onLinked={vi.fn()} />)
    const told = (await screen.findByLabelText('What Telegram is told')) as HTMLSelectElement
    fireEvent.change(told, { target: { value: 'full' } })
    await waitFor(() => expect(privacySet).toHaveBeenCalledWith('full'))
    const to = screen.getByLabelText('The orchestrator Telegram talks to') as HTMLSelectElement
    expect(to.value).toBe('')
    fireEvent.change(to, { target: { value: 'orch-home' } })
    await waitFor(() => expect(orchestratorSet).toHaveBeenCalledWith('orch-home'))
  })

  it('names the only orchestrator rather than offering a choice of one', async () => {
    status = { ...unlinked, linked: true, bot: 'my_bot', orchestrators: [{ project: 'orch-work', name: 'work' }] }
    render(<TelegramSetup onLinked={vi.fn()} />)
    expect(await screen.findByText('Talks to work, your only orchestrator.')).toBeTruthy()
    expect(screen.queryByLabelText('The orchestrator Telegram talks to')).toBeNull()
  })
})
