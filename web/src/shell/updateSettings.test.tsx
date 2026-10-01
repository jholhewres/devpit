import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { UpdateStatus } from '../gen/bindings'
import { SHOW_THE_UPDATE } from './UpdateCard'
import { UpdateSettings } from './UpdateSettings'

const heard: Record<string, (payload: UpdateStatus) => void> = {}
vi.mock('./window', () => ({
  onCarried: (name: string, then: (payload: UpdateStatus) => void) => {
    heard[name] = then
    return () => delete heard[name]
  },
}))

const closePrefs = vi.fn()
vi.mock('./useShell', () => ({ useShell: () => ({ closePrefs }) }))

const standing = vi.fn(async (): Promise<UpdateStatus> => ({ type: 'idle' }))
let downloaded: { data: UpdateStatus | null; error: string | null } = { data: null, error: null }
const download = vi.fn(async () => downloaded)
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => {
    const answer = await call()
    return answer && typeof answer === 'object' && 'error' in answer
      ? { ...(answer as object), loading: false }
      : { data: answer, error: null, loading: false }
  },
  commands: {
    appInfo: async () => ({ version: '0.1.30' }),
    updateStatus: () => standing(),
    updateCheck: async () => ({ type: 'idle' }),
    updateDownload: () => download(),
  },
}))

const offer: UpdateStatus = { type: 'available', version: '0.1.31', notes: '', kind: 'appImage', testFeed: false }

function say(status: UpdateStatus): void {
  act(() => heard['update:status']!(status))
}

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  downloaded = { data: null, error: null }
})

describe('the Updates row in Settings', () => {
  it('shows an update the automatic check already found', async () => {
    standing.mockResolvedValueOnce(offer)
    render(<UpdateSettings />)
    await waitFor(() => screen.getByText(/devpit 0\.1\.31 is out/))
    expect(screen.getByRole('button', { name: 'Update' })).toBeTruthy()
  })

  it('follows the download instead of saying "Working…" and then nothing', async () => {
    render(<UpdateSettings />)
    say(offer)
    say({ type: 'downloading', percent: 42 })
    expect(screen.getByText('Downloading the update · 42%')).toBeTruthy()
    say({ type: 'ready', version: '0.1.31' })
    expect(screen.getByText(/0\.1\.31 is downloaded/)).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Show the update' })).toBeTruthy()
  })

  it('hands over to the card once the download has started', async () => {
    const shown = vi.fn()
    window.addEventListener(SHOW_THE_UPDATE, shown)
    /* The command answers when the download is done; the states come first. */
    download.mockImplementationOnce(() => new Promise(() => undefined))
    render(<UpdateSettings />)
    say(offer)
    fireEvent.click(screen.getByRole('button', { name: 'Update' }))
    await waitFor(() => expect(download).toHaveBeenCalled())
    expect(closePrefs).not.toHaveBeenCalled()
    say({ type: 'downloading', percent: 0 })
    expect(closePrefs).toHaveBeenCalledTimes(1)
    expect(shown).toHaveBeenCalledTimes(1)
    window.removeEventListener(SHOW_THE_UPDATE, shown)
  })

  it('keeps a refusal here, where it was asked', async () => {
    downloaded = { data: null, error: 'this build was not installed from a devpit release' }
    render(<UpdateSettings />)
    say(offer)
    fireEvent.click(screen.getByRole('button', { name: 'Update' }))
    await waitFor(() => screen.getByText(/not installed from a devpit release/))
    expect(closePrefs).not.toHaveBeenCalled()
  })

  it('takes you to the card when the update is waiting there', async () => {
    standing.mockResolvedValueOnce({ type: 'manualInstall', command: 'sudo apt install x', path: '/x.deb' })
    render(<UpdateSettings />)
    fireEvent.click(await waitFor(() => screen.getByRole('button', { name: 'Show the update' })))
    expect(closePrefs).toHaveBeenCalledTimes(1)
  })
})
