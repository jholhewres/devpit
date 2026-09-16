import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { UpdateStatus } from '../gen/bindings'
import { counting, RELEASE_PAGE, UpdateCard, waitingFor } from './UpdateCard'

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
const checked = vi.fn(async () => ({ status: 'ok', data: { type: 'checking' } }))
const chose = vi.fn(async (_choice: string) => ({ status: 'ok', data: null }))
type Blocking = { id: string; title: string }
let busy: { runs: Blocking[]; turns: Blocking[]; keeps: Blocking[] } = { runs: [], turns: [], keeps: [] }
vi.mock('./live', () => ({
  ask: async (call: () => Promise<{ data?: unknown }>) => {
    const answer = (await call()) as { data?: unknown }
    return { data: answer?.data ?? null, error: null, loading: false }
  },
  commands: {
    updateDownload: () => download(),
    updatePackage: () => packaged(),
    updateInstall: () => installed(),
    updateCheck: () => checked(),
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
  busy = { runs: [], turns: [], keeps: [] }
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
    expect(screen.queryByText(/The clipboard refused it/)).toBeNull()
  })

  it('says so when the clipboard refuses the command', async () => {
    Object.assign(navigator, { clipboard: { writeText: vi.fn(async () => Promise.reject(new Error('no'))) } })
    render(<UpdateCard />)
    say({ type: 'manualInstall', command: 'sudo x', path: '/c/devpit.deb' })
    fireEvent.click(screen.getByRole('button', { name: 'Copy command' }))
    expect(await screen.findByText(/The clipboard refused it/)).toBeTruthy()
  })

  /* A build from `make dev` or a tarball: the files, not a button that can
     only refuse. */
  it('points a build devpit does not install over at the release page', () => {
    render(<UpdateCard />)
    say({ type: 'available', version: '0.2.0', notes: '', kind: 'unmanaged', testFeed: false })
    expect(screen.queryByRole('button', { name: 'Update' })).toBeNull()
    expect(screen.getByRole('link', { name: 'Release page' }).getAttribute('href')).toBe(RELEASE_PAGE)
  })

  it('still says test feed once a download from it was refused', () => {
    render(<UpdateCard />)
    say({ type: 'available', version: '0.2.0', notes: '', kind: 'appImage', testFeed: true })
    say({ type: 'failed', message: 'this offer came from a test feed', recoverable: true })
    expect(screen.getByText('test feed')).toBeTruthy()
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
    busy = {
      runs: [{ id: 'run_1', title: 'Fix the parser' }],
      turns: [],
      keeps: [{ id: 'leaf_1', title: 'Wire the board, in a terminal' }],
    }
    render(<UpdateCard />)
    ready()
    fireEvent.click(screen.getByText('Restart now'))

    expect(await screen.findByText(/Fix the parser/)).toBeTruthy()
    expect(screen.getByText('Keeps running: Wire the board, in a terminal.')).toBeTruthy()
    expect(installed).not.toHaveBeenCalled()

    // The app stops the work and installs; the window does not race it with
    // an install of its own.
    fireEvent.click(screen.getByText('Stop it and restart'))
    await waitFor(() => expect(chose).toHaveBeenCalledWith('stopIt'))
    expect(installed).not.toHaveBeenCalled()
  })

  it('lets the update wait for the work instead', async () => {
    busy = { runs: [], turns: [{ id: 'conv_1', title: 'Ship it' }], keeps: [] }
    render(<UpdateCard />)
    ready()
    fireEvent.click(screen.getByText('Restart now'))

    fireEvent.click(await screen.findByText('When it is done'))
    await waitFor(() => expect(chose).toHaveBeenCalledWith('whenItIsDone'))
    expect(installed).not.toHaveBeenCalled()
  })
})

/* The only overlay that appears on its own, so the only one a focus has to
   hold back. It waits rather than being dismissed: an update nobody was told
   about is not an update that went away. */
describe('an update while a focus is on', () => {
  afterEach(() => delete document.documentElement.dataset.headsDown)

  it('says nothing during a focus, and says it when the focus ends', async () => {
    document.documentElement.dataset.headsDown = 'prj_1:1700000000'
    render(<UpdateCard />)
    say({ type: 'available', version: '0.2.0', notes: '', kind: 'appImage', testFeed: false } as UpdateStatus)
    expect(screen.queryByText('Update ready')).toBeNull()
    expect(screen.queryByRole('status', { name: 'Update' })).toBeNull()

    delete document.documentElement.dataset.headsDown
    await waitFor(() => expect(screen.getByRole('status', { name: 'Update' })).toBeTruthy())
  })
})

describe('an update waiting for the work to end', () => {
  it('says what it is waiting for, and since when, and Cancel is told to the app', async () => {
    render(<UpdateCard />)
    say({ type: 'waiting', runs: 2, turns: 1, since: Date.now() / 1000 - 180 } as UpdateStatus)
    expect(screen.getByText('Update waiting')).toBeTruthy()
    expect(screen.getByText(/2 runs and 1 turn are done\. Waiting for 3 min\./)).toBeTruthy()

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    await waitFor(() => expect(chose).toHaveBeenCalledWith('later'))
  })

  it('stops the work and updates now, when asked from the wait', async () => {
    render(<UpdateCard />)
    say({ type: 'waiting', runs: 1, turns: 0, since: Date.now() / 1000 } as UpdateStatus)
    fireEvent.click(screen.getByRole('button', { name: 'Stop them and update now' }))
    await waitFor(() => expect(chose).toHaveBeenCalledWith('stopIt'))
  })

  it('counts the wait in minutes, then hours', () => {
    expect(waitingFor(100, 110)).toBe('for less than a minute')
    expect(waitingFor(0, 125)).toBe('for 2 min')
    expect(waitingFor(0, 3900)).toBe('for 1 h 5 min')
  })

  /* Nothing arrives between the choice and the work ending, so the card has to
     move its own clock — it said "for less than a minute" for as long as it was
     left open. */
  it('keeps counting while it waits, and only while it waits', () => {
    vi.useFakeTimers()
    try {
      const started = Date.now() / 1000
      render(<UpdateCard />)
      say({ type: 'waiting', runs: 1, turns: 0, since: started } as UpdateStatus)
      expect(screen.getByText(/Waiting for less than a minute\./)).toBeTruthy()

      act(() => void vi.advanceTimersByTime(90_000))
      expect(screen.getByText(/Waiting for 1 min\./)).toBeTruthy()

      act(() => void vi.advanceTimersByTime(4 * 60_000))
      expect(screen.getByText(/Waiting for 5 min\./)).toBeTruthy()

      // A state with nothing to count does not keep a timer running.
      expect(counting({ type: 'waiting', runs: 0, turns: 0, since: 0 } as UpdateStatus)).toBe(true)
      expect(counting({ type: 'ready', version: '0.2.0' } as UpdateStatus)).toBe(false)
      expect(counting({ type: 'installing' } as UpdateStatus)).toBe(false)
      expect(counting(null)).toBe(false)
    } finally {
      vi.useRealTimers()
    }
  })

  /* A refusal before the install commits leaves the offer good, so the card
     has to offer a way back: Failed with no action meant going to Settings to
     retry something the card already knew about. */
  it('offers a way back when the failure came before the install committed', async () => {
    render(<UpdateCard />)
    say({ type: 'failed', message: 'the download was refused', recoverable: true } as UpdateStatus)
    fireEvent.click(screen.getByRole('button', { name: 'Check again' }))
    await waitFor(() => expect(checked).toHaveBeenCalled())
  })

  it('offers nothing once the install has committed, because there is nothing to retry', () => {
    render(<UpdateCard />)
    say({ type: 'failed', message: 'the installer refused', recoverable: false } as UpdateStatus)
    expect(screen.queryByRole('button', { name: 'Check again' })).toBeNull()
  })

  /* The review's case: closing a ready update only hid the card, and the app
     kept the update pending. */
  it('tells the app when a ready update is closed', async () => {
    render(<UpdateCard />)
    say({ type: 'ready', version: '0.2.0', kind: 'appImage' } as UpdateStatus)
    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    await waitFor(() => expect(chose).toHaveBeenCalledWith('later'))
  })

  it('does not tell the app anything when an offer is closed', () => {
    render(<UpdateCard />)
    say({ type: 'available', version: '0.2.0', notes: '', kind: 'appImage', testFeed: false })
    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(chose).not.toHaveBeenCalled()
  })
})
