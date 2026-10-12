import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { TelegramStatus } from '../gen/bindings'

const unlinked: TelegramStatus = {
  linked: false,
  bot: null,
  code: null,
  link: null,
  privacy: 'minimal',
  orchestrator: null,
  orchestrators: [],
  devpitBot: false,
  throughDevpit: false,
}
let status: TelegramStatus = unlinked
const linked = vi.fn()
const privacySet = vi.fn()
const orchestratorSet = vi.fn()
const usedDevpit = vi.fn()
const opened = vi.fn()
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: {
    telegramStatus: () => status,
    telegramLink: (token: string) => (linked(token), { ...unlinked, bot: 'my_bot', code: 'A1B2C3', link: 'https://t.me/my_bot?start=A1B2C3' }),
    telegramUseDevpit: () => (usedDevpit(), { ...status, bot: 'devpit_app_bot', code: 'ABCD-EFGH', link: 'https://t.me/devpit_app_bot?start=ABCD-EFGH' }),
    telegramUnlink: () => unlinked,
    telegramPrivacySet: (privacy: string) => (privacySet(privacy), { ...status, privacy }),
    telegramOrchestratorSet: (project: string | null) => (orchestratorSet(project), { ...status, orchestrator: project }),
    urlOpen: (url: string) => (opened(url), { path: '' }),
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

  it('offers devpit’s bot only when devpit.app says it has one', async () => {
    status = unlinked
    render(<TelegramSetup onLinked={vi.fn()} />)
    await screen.findByPlaceholderText('123456:ABC…')
    expect(screen.queryByText('Use devpit’s bot')).toBeNull()
  })

  it('opens devpit’s bot with a single-use code', async () => {
    status = { ...unlinked, devpitBot: true }
    render(<TelegramSetup onLinked={vi.fn()} />)
    fireEvent.click(await screen.findByText('Use devpit’s bot'))
    await waitFor(() => expect(usedDevpit).toHaveBeenCalled())
    expect(await screen.findByText('/start ABCD-EFGH')).toBeTruthy()
    expect(opened).toHaveBeenCalledWith('https://t.me/devpit_app_bot?start=ABCD-EFGH')
  })

  it('offers no whole answers through devpit’s bot', async () => {
    status = { ...unlinked, linked: true, bot: 'devpit_app_bot', throughDevpit: true }
    render(<TelegramSetup onLinked={vi.fn()} />)
    const told = (await screen.findByLabelText('What Telegram is told')) as HTMLSelectElement
    expect([...told.options].map((one) => one.value)).toEqual(['minimal', 'titles'])
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
