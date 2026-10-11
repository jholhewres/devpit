import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { CalendarFeed } from '../gen/bindings'

let feed: CalendarFeed
const set = vi.fn()
const reset = vi.fn()

vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    calendarRead: async () => feed,
    calendarSet: async (on: boolean, titles: boolean) => {
      set(on, titles)
      feed = { ...feed, link: on ? (feed.link ?? 'https://devpit.app/cal/first.ics') : null, titles }
      return feed
    },
    calendarReset: async () => {
      reset()
      feed = { ...feed, link: 'https://devpit.app/cal/second.ics' }
      return feed
    },
  },
}))

import { CalendarSettings } from './CalendarSettings'

beforeEach(() => {
  feed = { link: 'https://devpit.app/cal/first.ics', titles: false, signedIn: true }
})

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

describe('the calendar on devpit.app in Settings', () => {
  it('copies the link', async () => {
    const writeText = vi.fn(async () => undefined)
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    render(<CalendarSettings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Copy link' }))
    expect(writeText).toHaveBeenCalledWith('https://devpit.app/cal/first.ics')
    await screen.findByRole('button', { name: 'Copied' })
  })

  it('keeps titles off until they are asked for', async () => {
    render(<CalendarSettings />)
    const titles = (await screen.findByRole('checkbox', { name: /Show titles/ })) as HTMLInputElement
    expect(titles.checked).toBe(false)
    fireEvent.click(titles)
    await waitFor(() => expect(set).toHaveBeenCalledWith(true, true))
  })

  it('resets the link to a new one', async () => {
    render(<CalendarSettings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Reset link' }))
    await screen.findByText('https://devpit.app/cal/second.ics')
    expect(reset).toHaveBeenCalledOnce()
  })

  it('ends the link from its switch', async () => {
    render(<CalendarSettings />)
    fireEvent.click(await screen.findByRole('switch', { name: 'Calendar' }))
    await waitFor(() => expect(set).toHaveBeenCalledWith(false, false))
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Copy link' })).toBeNull())
  })

  it('cannot be turned on before signing in to devpit.app', async () => {
    feed = { link: null, titles: false, signedIn: false }
    render(<CalendarSettings />)
    const toggle = (await screen.findByRole('switch', { name: 'Calendar' })) as HTMLButtonElement
    expect(toggle.disabled).toBe(true)
    expect(screen.queryByRole('button', { name: 'Copy link' })).toBeNull()
  })
})
