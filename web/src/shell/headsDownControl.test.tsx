import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { HeadsDown as Stored } from '../gen/bindings'
import { HeadsDown } from './HeadsDownControl'

let stored: Stored | null = null
const written: (string | null)[] = []
const lengths: number[] = []
let offered = true

vi.mock('./live', () => ({
  ask: async (call: () => Promise<{ data?: unknown }>) => {
    const answer = (await call()) as { data?: unknown }
    return { data: answer?.data ?? null, error: null, loading: false }
  },
  commands: {
    focusRead: async () => ({ status: 'ok', data: stored }),
    focusWaiting: async () => ({ notices: [], more: false }),
    /* The control is off unless Settings turned it on, so every test here
       says it was turned on. The one that proves it stays hidden says so. */
    settingsRead: async () => ({ status: 'ok', data: { focusMode: offered } }),
    runsList: async () => ({ runs: [], next: null }),
    focusWrite: async (projectId: string | null, minutes: number) => {
      lengths.push(minutes)
      written.push(projectId)
      stored = projectId ? { projectId, since: Date.now() / 1000, until: null } : null
      return { status: 'ok', data: stored }
    },
  },
}))

afterEach(() => {
  cleanup()
  stored = null
  written.length = 0
  lengths.length = 0
  offered = true
  delete document.documentElement.dataset.headsDown
  vi.useRealTimers()
})

describe('going into a focus', () => {
  it('shows nothing at all without a project open', async () => {
    const { container } = render(<HeadsDown projectId={null} />)
    await waitFor(() => expect(container.firstChild).toBeNull())
  })

  it('begins one on a click and ends it on the next', async () => {
    render(<HeadsDown projectId="prj_1" />)
    const button = await screen.findByRole('button', { name: /Focus/ })
    expect(button.getAttribute('aria-pressed')).toBe('false')

    fireEvent.click(button)
    await waitFor(() => expect(written).toEqual(['prj_1']))
    await waitFor(() => expect(button.getAttribute('aria-pressed')).toBe('true'))

    fireEvent.click(button)
    await waitFor(() => expect(written).toEqual(['prj_1', null]))
    await waitFor(() => expect(button.getAttribute('aria-pressed')).toBe('false'))
  })

  /* The shell reads it in CSS, the way it reads the theme, so AppShell does
     not have to hold this state to pass it down. */
  it('stamps the root element while it is on, and unstamps it after', async () => {
    render(<HeadsDown projectId="prj_1" />)
    expect(document.documentElement.dataset.headsDown).toBeUndefined()

    fireEvent.click(await screen.findByRole('button', { name: /Focus/ }))
    // The whole focus, not a flag: the bell reads the project and the second
    // it began off this, and a flag would leave it asking the app again.
    await waitFor(() =>
      expect(document.documentElement.dataset.headsDown).toMatch(/^prj_1:\d+/),
    )

    fireEvent.click(screen.getByRole('button', { name: /Focus/ }))
    await waitFor(() => expect(document.documentElement.dataset.headsDown).toBeUndefined())
  })

  /* Announced and heard in the same file, because the opposite is how ⌘P came
     to be printed on a button nothing listened for. */
  it('hears the key it announces', async () => {
    render(<HeadsDown projectId="prj_1" />)
    expect((await screen.findByRole('button', { name: /Focus/ })).getAttribute('title')).toContain(
      '⇧⌘F',
    )

    fireEvent.keyDown(window, { key: 'F', shiftKey: true, metaKey: true })
    await waitFor(() => expect(written).toEqual(['prj_1']))
  })

  it('reads back a focus that was already on when the window opened', async () => {
    stored = { projectId: 'prj_1', since: Date.now() / 1000 - 300, until: null }
    render(<HeadsDown projectId="prj_1" />)
    await waitFor(() =>
      expect(screen.getByRole('button', { name: /Focus · 5 min/ })).toBeTruthy(),
    )
  })

  /* The minutes moving while it is on belongs to the pill, with the counter
     it is drawn from — US-007. Testing it here meant installing fake timers
     after the component had already registered a real interval, which proves
     nothing about either. */
})

describe('opening another project', () => {
  /* A focus is on one project by definition. Asking whether they meant it
     would be asking about the thing they just did. */
  it('ends the focus and says what it held', async () => {
    stored = { projectId: 'prj_1', since: Date.now() / 1000 - 60, until: null }
    const { rerender } = render(<HeadsDown projectId="prj_1" />)
    await waitFor(() => expect(screen.getByRole('button', { name: /Focus · 1 min/ })).toBeTruthy())

    rerender(<HeadsDown projectId="prj_2" />)
    await waitFor(() => expect(written).toEqual([null]))
    await waitFor(() => expect(screen.getByRole('dialog', { name: 'Focus ended' })).toBeTruthy())
  })

  it('leaves it alone while the project it is on stays open', async () => {
    stored = { projectId: 'prj_1', since: Date.now() / 1000, until: null }
    const { rerender } = render(<HeadsDown projectId="prj_1" />)
    await waitFor(() => expect(screen.getByRole('button', { name: /Focus ·/ })).toBeTruthy())

    rerender(<HeadsDown projectId="prj_1" waiting={2} />)
    expect(written).toEqual([])
  })
})

describe('what the pill says while a focus is on', () => {
  it('counts what is waiting outside, and says nothing when none is', async () => {
    stored = { projectId: 'prj_1', since: Date.now() / 1000 - 120, until: null }
    const { rerender } = render(<HeadsDown projectId="prj_1" waiting={0} />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Focus · 2 min' })).toBeTruthy())

    rerender(<HeadsDown projectId="prj_1" waiting={3} />)
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Focus · 2 min · 3 outside' })).toBeTruthy(),
    )
  })

  /* One meaning. Sharing this click with a peek left the button with no way
     out of the focus at exactly the moment somebody would want one, which the
     end-to-end test found by trying to leave. What is held is in the bell
     beside it, under its own heading. */
  it('leaves the focus even while something is held', async () => {
    stored = { projectId: 'prj_1', since: Date.now() / 1000, until: null }
    render(<HeadsDown projectId="prj_1" waiting={2} />)
    await waitFor(() => expect(screen.getByRole('button', { name: /outside/ })).toBeTruthy())

    fireEvent.click(screen.getByRole('button', { name: /outside/ }))
    await waitFor(() => expect(written).toEqual([null]))
  })
})

describe('a length given to a focus', () => {
  it('sends none unless one was picked', async () => {
    render(<HeadsDown projectId="prj_1" />)
    fireEvent.click(await screen.findByRole('button', { name: 'Focus' }))
    await waitFor(() => expect(lengths).toEqual([0]))
  })

  /* Offered, never imposed: the evidence for a fixed break is contradictory
     and only about students, so the default is no end at all. */
  it('sends the one that was picked', async () => {
    render(<HeadsDown projectId="prj_1" />)
    fireEvent.change(await screen.findByLabelText('Focus length'), { target: { value: '25' } })
    fireEvent.click(screen.getByRole('button', { name: 'Focus' }))
    await waitFor(() => expect(lengths).toEqual([25]))
  })

  it('offers no length while a focus is already on', async () => {
    stored = { projectId: 'prj_1', since: Date.now() / 1000, until: null }
    render(<HeadsDown projectId="prj_1" />)
    await waitFor(() => expect(screen.getByRole('button', { name: /Focus ·/ })).toBeTruthy())
    expect(screen.queryByLabelText('Focus length')).toBeNull()
  })
})

/* Unfinished, so it is not in everybody's top bar: Settings turns it on. */
describe('before it is turned on', () => {
  it('is not in the top bar at all', async () => {
    offered = false
    render(<HeadsDown projectId="prj_1" />)
    await waitFor(() => expect(screen.queryByRole('button', { name: /Focus/ })).toBeNull())
  })
})
