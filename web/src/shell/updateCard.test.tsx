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
const installed = vi.fn(async () => ({ status: 'ok', data: null }))
const chose = vi.fn(async (_choice: string) => ({ status: 'ok', data: null }))
let busy: { runs: { id: string; title: string }[]; turns: { id: string; title: string }[] } = {
  runs: [],
  turns: [],
}
vi.mock('./live', () => ({
  ask: async (call: () => Promise<{ data?: unknown }>) => {
    const answer = (await call()) as { data?: unknown }
    return { data: answer?.data ?? null, error: null, loading: false }
  },
  commands: {
    updateDownload: () => download(),
    updatePackage: () => packaged(),
    updateInstall: () => installed(),
    updateRunning: async () => ({ status: 'ok', data: busy }),
    updateChoose: (choice: string) => chose(choice),
  },
}))

/* The app pushes these in from outside React, so the render has to be let
   through before anything is asserted about it. */
function say(status: UpdateStatus): void {
  act(() => heard['update:status']!(status))
}

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  busy = { runs: [], turns: [] }
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

  it('offers the version, says the terminals keep running, and downloads on the click', () => {
    render(<UpdateCard />)
    say({
      type: 'available',
      version: '0.2.0',
      notes: 'what changed',
      kind: 'appImage',
      testFeed: false,
    })

    expect(screen.getByText('Update available')).toBeTruthy()
    expect(screen.getByText('devpit 0.2.0 is ready.')).toBeTruthy()
    expect(screen.getByText('Your terminals keep running.')).toBeTruthy()
    // The notes are one click away, not the first thing in the corner.
    expect(screen.queryByText('what changed')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Release notes' }))
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

  it('goes away on close, and comes back when the state changes', () => {
    render(<UpdateCard />)
    say({ type: 'available', version: '0.2.0', notes: '', kind: 'appImage', testFeed: false })
    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(screen.queryByRole('status')).toBeNull()

    say({ type: 'ready', version: '0.2.0' } as UpdateStatus)
    expect(screen.getByText('devpit 0.2.0 is ready to install.')).toBeTruthy()
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
    expect(screen.getByText(/devpit never runs an install command/)).toBeTruthy()

    fireEvent.click(screen.getByRole('button', { name: 'Copy command' }))
    await waitFor(() => expect(packaged).toHaveBeenCalled())
    expect(written).toHaveBeenCalledWith("sudo /usr/bin/apt install '/c/devpit.deb'")
  })
})

describe('restarting into the update', () => {
  const ready = (): void =>
    say({ type: 'ready', version: '0.2.0', kind: 'appImage' } as UpdateStatus)

  it('goes straight in when nothing is running', async () => {
    render(<UpdateCard />)
    ready()
    fireEvent.click(screen.getByText('Restart now'))
    await waitFor(() => expect(installed).toHaveBeenCalled())
  })

  /* The question is about the two runs, not about the update: the card names
     them and installs nothing until it is answered. */
  it('names what is running, and waits for an answer', async () => {
    busy = { runs: [{ id: 'run_1', title: 'Fix the parser' }], turns: [] }
    render(<UpdateCard />)
    ready()
    fireEvent.click(screen.getByText('Restart now'))

    expect(await screen.findByText(/Fix the parser/)).toBeTruthy()
    expect(installed).not.toHaveBeenCalled()

    fireEvent.click(screen.getByText('Stop it and restart'))
    await waitFor(() => expect(chose).toHaveBeenCalledWith('stopIt'))
    await waitFor(() => expect(installed).toHaveBeenCalled())
  })

  it('lets the update wait for the work instead', async () => {
    busy = { runs: [], turns: [{ id: 'conv_1', title: 'Ship it' }] }
    render(<UpdateCard />)
    ready()
    fireEvent.click(screen.getByText('Restart now'))

    fireEvent.click(await screen.findByText('When it is done'))
    await waitFor(() => expect(chose).toHaveBeenCalledWith('whenItIsDone'))
    expect(installed).not.toHaveBeenCalled()
  })
})
