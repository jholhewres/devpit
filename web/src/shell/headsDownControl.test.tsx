import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { HeadsDown as Stored } from '../gen/bindings'
import { HeadsDown } from './HeadsDown'

let stored: Stored | null = null
const written: (string | null)[] = []

vi.mock('./live', () => ({
  ask: async (call: () => Promise<{ data?: unknown }>) => {
    const answer = (await call()) as { data?: unknown }
    return { data: answer?.data ?? null, error: null, loading: false }
  },
  commands: {
    focusRead: async () => ({ status: 'ok', data: stored }),
    focusWrite: async (projectId: string | null) => {
      written.push(projectId)
      stored = projectId ? { projectId, since: Date.now() / 1000 } : null
      return { status: 'ok', data: stored }
    },
  },
}))

afterEach(() => {
  cleanup()
  stored = null
  written.length = 0
  delete document.documentElement.dataset.headsDown
  vi.useRealTimers()
})

describe('going into a focus', () => {
  it('shows nothing at all without a project open', () => {
    const { container } = render(<HeadsDown projectId={null} />)
    expect(container.firstChild).toBeNull()
  })

  it('begins one on a click and ends it on the next', async () => {
    render(<HeadsDown projectId="prj_1" />)
    const button = screen.getByRole('button', { name: /Focus/ })
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

    fireEvent.click(screen.getByRole('button', { name: /Focus/ }))
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
    expect(screen.getByRole('button', { name: /Focus/ }).getAttribute('title')).toContain('⇧⌘F')

    fireEvent.keyDown(window, { key: 'F', shiftKey: true, metaKey: true })
    await waitFor(() => expect(written).toEqual(['prj_1']))
  })

  it('reads back a focus that was already on when the window opened', async () => {
    stored = { projectId: 'prj_1', since: Date.now() / 1000 - 300 }
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
