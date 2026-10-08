import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { TelegramStatus } from '../gen/bindings'

let status: TelegramStatus = { linked: false, bot: null, code: null, link: null, titles: false }
const linked = vi.fn()
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: {
    telegramStatus: () => status,
    telegramLink: (token: string) => (linked(token), { linked: false, bot: 'my_bot', code: 'A1B2C3', link: 'https://t.me/my_bot?start=A1B2C3', titles: false }),
    telegramUnlink: () => ({ linked: false, bot: null, code: null, link: null, titles: false }),
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
    status = { linked: true, bot: 'my_bot', code: null, link: null, titles: false }
    render(<TelegramSetup onLinked={vi.fn()} />)
    expect(await screen.findByText('Telegram, @my_bot')).toBeTruthy()
    expect(screen.getByText('Unlink')).toBeTruthy()
  })
})
