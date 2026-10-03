import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { RemoteDevice, RemoteView, Transcribing } from '../gen/bindings'
import { PrefSwitch } from './PrefSwitch'
import { RemoteSettings } from './RemoteSettings'
import { VoiceSettings } from './VoiceSettings'

const allowed = vi.fn()
let whisper: string | null = null

const device = (over: Partial<RemoteDevice> = {}): RemoteDevice => ({
  id: 'dev_1',
  name: 'iPhone',
  typing: false,
  answering: true,
  pairedAt: 1,
  lastSeen: null,
  connected: false,
  ...over,
})

const view = (): RemoteView => ({
  enabled: true,
  tailscale: { installed: true, running: true, name: 'box.tail1.ts.net', https: true, login: 'me@example.com', ip: null },
  address: 'https://box.tail1.ts.net:8443/',
  problem: null,
  devices: [device()],
})

const voice = (): Transcribing => ({ engine: 'local', model: '', language: '', url: '', keySet: false, local: whisper })

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
vi.mock('./window', () => ({ onCarried: () => () => undefined }))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    remoteRead: async () => view(),
    remoteAllow: async (id: string, typing: boolean, answering: boolean) => (allowed(id, typing, answering), view()),
    remoteForget: async () => view(),
    transcribeRead: async () => voice(),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  whisper = null
})

describe('a settings switch', () => {
  it('flips from the switch alone, not from the words beside it', () => {
    const flip = vi.fn()
    render(<PrefSwitch on={false} onFlip={flip} title="Open at login" said="devpit starts with your desktop." more="One copy runs at a time." />)
    fireEvent.click(screen.getByText('Open at login'))
    fireEvent.click(screen.getByText('devpit starts with your desktop.'))
    fireEvent.click(screen.getByText('How it works'))
    expect(flip).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('switch', { name: 'Open at login' }))
    expect(flip).toHaveBeenCalledOnce()
  })

  it('keeps the long half folded until it is asked for', () => {
    render(<PrefSwitch on onFlip={vi.fn()} title="Island" said="A small window." more="Drag it to another screen." />)
    expect(screen.getByText('Drag it to another screen.').closest('details')?.open).toBe(false)
  })
})

describe('Remote in Settings', () => {
  it('keeps the numbered steps in the same card as its switch', async () => {
    render(<RemoteSettings />)
    const toggle = await screen.findByRole('switch', { name: 'Remote' })
    const card = toggle.closest('.pref') as HTMLElement
    expect(within(card).getAllByRole('listitem')).toHaveLength(4)
    expect(card.querySelector('ol.prefhow')).not.toBeNull()
  })

  it('copies the address', async () => {
    const writeText = vi.fn(async () => undefined)
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    render(<RemoteSettings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Copy' }))
    expect(writeText).toHaveBeenCalledWith('https://box.tail1.ts.net:8443/')
    await screen.findByRole('button', { name: 'Copied' })
  })

  it('grants typing from its chip and leaves answering as it was', async () => {
    render(<RemoteSettings />)
    const type = await screen.findByRole('button', { name: 'type' })
    expect(type.getAttribute('aria-pressed')).toBe('false')
    expect(screen.getByRole('button', { name: 'answer' }).getAttribute('aria-pressed')).toBe('true')
    fireEvent.click(type)
    await waitFor(() => expect(allowed).toHaveBeenCalledWith('dev_1', true, true))
  })

  it('shows watching as always on, with nothing to click', async () => {
    render(<RemoteSettings />)
    await screen.findByText('watch')
    expect(screen.queryByRole('button', { name: 'watch' })).toBeNull()
    expect(screen.queryByRole('checkbox')).toBeNull()
  })
})

describe('Voice in Settings', () => {
  it('looks for a whisper again without leaving Settings', async () => {
    render(<VoiceSettings />)
    const again = await screen.findByRole('button', { name: 'Check again' })
    whisper = 'whisper-cli'
    fireEvent.click(again)
    await screen.findByText('Found whisper-cli.')
  })

  it('says so when it looked and there is still none', async () => {
    render(<VoiceSettings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Check again' }))
    await screen.findByText('Still no whisper on this machine.')
  })
})
