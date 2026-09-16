import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { UpdateStatus } from '../gen/bindings'
import { UpdateCard } from './UpdateCard'

const heard: Record<string, (payload: UpdateStatus) => void> = {}
vi.mock('./window', () => ({
  onCarried: (name: string, then: (payload: UpdateStatus) => void) => {
    heard[name] = then
    return () => delete heard[name]
  },
}))

const download = vi.fn(async () => ({ status: 'ok', data: null }))
const packaged = vi.fn(async () => ({ status: 'ok', data: "sudo /usr/bin/apt install '/c/devpit.deb'" }))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<{ data?: unknown }>) => {
    const answer = (await call()) as { data?: unknown }
    return { data: answer?.data ?? null, error: null, loading: false }
  },
  commands: { updateDownload: () => download(), updatePackage: () => packaged() },
}))

/* The app pushes these in from outside React, so the render has to be let
   through before anything is asserted about it. */
function say(status: UpdateStatus): void {
  act(() => heard['update:status']!(status))
}

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

describe('the update card', () => {
  it('says nothing until there is something to say', () => {
    render(<UpdateCard />)
    expect(screen.queryByRole('status')).toBeNull()

    say({ type: 'checking' })
    expect(screen.queryByRole('status')).toBeNull()

    say({ type: 'idle' })
    expect(screen.queryByRole('status')).toBeNull()
  })

  it('offers the version and its notes, and downloads on the click', () => {
    render(<UpdateCard />)
    say({
      type: 'available',
      version: '0.2.0',
      notes: 'what changed',
      kind: 'appImage',
      testFeed: false,
    })

    expect(screen.getByText('devpit 0.2.0 is out')).toBeTruthy()
    expect(screen.getByText('what changed')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Update' }))
    expect(download).toHaveBeenCalled()
  })

  /* An offer nobody can install must not look like one they can. */
  it('says when the offer came from a test feed', () => {
    render(<UpdateCard />)
    say({
      type: 'available',
      version: '0.2.0',
      notes: '',
      kind: 'appImage',
      testFeed: true,
    })
    expect(screen.getByText('test feed')).toBeTruthy()
  })

  it('draws the progress while it downloads, and offers no second Update', () => {
    render(<UpdateCard />)
    say({ type: 'downloading', percent: 42 })

    expect(screen.getByText('Downloading the update')).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Update' })).toBeNull()
  })

  it('goes away on Later, and comes back when the state changes', () => {
    render(<UpdateCard />)
    say({ type: 'available', version: '0.2.0', notes: '', kind: 'appImage', testFeed: false })
    fireEvent.click(screen.getByRole('button', { name: 'Later' }))
    expect(screen.queryByRole('status')).toBeNull()

    say({ type: 'ready', version: '0.2.0' })
    expect(screen.getByText('devpit 0.2.0 is ready')).toBeTruthy()
  })

  it('says what failed', () => {
    render(<UpdateCard />)
    say({ type: 'failed', message: 'the download failed', recoverable: true })
    expect(screen.getByText('the download failed')).toBeTruthy()
  })

  /* A package devpit will not install: the path, the note, and a command asked
     for again at the moment it is copied. */
  it('shows a package to install by hand, and asks again before copying', async () => {
    const written = vi.fn()
    Object.assign(navigator, { clipboard: { writeText: written } })
    render(<UpdateCard />)

    say({
      type: 'manualInstall',
      command: "sudo /usr/bin/apt install '/c/devpit.deb'",
      path: '/c/devpit.deb',
    })

    expect(screen.getByText('Install this package yourself')).toBeTruthy()
    expect(screen.getByText('/c/devpit.deb')).toBeTruthy()
    expect(screen.getByText(/devpit never runs it/)).toBeTruthy()

    fireEvent.click(screen.getByRole('button', { name: 'Copy command' }))
    await waitFor(() => expect(packaged).toHaveBeenCalled())
    expect(written).toHaveBeenCalledWith("sudo /usr/bin/apt install '/c/devpit.deb'")
  })
})
