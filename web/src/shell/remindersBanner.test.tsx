import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Reminder, Reminders } from '../gen/bindings'
import { RemindersBanner } from './RemindersBanner'
import { laterOptions } from './useReminders'

const heard: Record<string, (payload: unknown) => void> = {}
vi.mock('./window', () => ({
  onCarried: (name: string, then: (payload: unknown) => void) => {
    heard[name] = then
    return () => delete heard[name]
  },
}))

const reminder = (cardId: string, title: string, extra: Partial<Reminder> = {}): Reminder => ({
  cardId,
  projectId: 'prj_api',
  project: 'asc-api',
  title,
  dueAt: Date.now() / 1000 - 60,
  timed: true,
  ...extra,
})

let pending: Reminder[] = []
const snoozed = vi.fn()
const finished = vi.fn()
const rearmed = vi.fn(async () => undefined)
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    remindersPending: async (): Promise<Reminders> => ({ reminders: pending }),
    reminderSnooze: async (cardId: string, until: number): Promise<Reminders> => {
      snoozed(cardId, until)
      pending = pending.filter((one) => one.cardId !== cardId)
      return { reminders: pending }
    },
    reminderDone: async (cardId: string): Promise<Reminders> => {
      finished(cardId)
      pending = pending.filter((one) => one.cardId !== cardId)
      return { reminders: pending }
    },
    remindersRearm: () => rearmed(),
  },
}))

const opened = { setProject: vi.fn(), show: vi.fn(), openCard: vi.fn() }
vi.mock('./useShell', () => ({ useShell: () => ({ ...opened, project: { id: 'prj_web' } }) }))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  pending = []
})

describe('a reminder that went off', () => {
  it('shows nothing when nothing went off', async () => {
    const { container } = render(<RemindersBanner />)
    await act(async () => undefined)
    expect(container.innerHTML).toBe('')
  })

  it('stays until it is put off or done — there is no close', async () => {
    pending = [reminder('card_1', 'Review the PR')]
    render(<RemindersBanner />)
    await waitFor(() => screen.getByText('Review the PR'))
    expect(screen.queryByRole('button', { name: /close|dismiss/i })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: '1 hour' }))
    await waitFor(() => expect(snoozed).toHaveBeenCalledWith('card_1', expect.any(Number)))
    const until = snoozed.mock.calls[0]![1] as number
    expect(until - Date.now() / 1000).toBeGreaterThan(3500)
    await waitFor(() => expect(screen.queryByText('Review the PR')).toBeNull())
  })

  it('is done, and opens its card in its own project', async () => {
    pending = [reminder('card_1', 'Review the PR')]
    render(<RemindersBanner />)
    fireEvent.click(await waitFor(() => screen.getByTitle('Open the card')))
    expect(opened.setProject).toHaveBeenCalledWith('prj_api')
    expect(opened.openCard).toHaveBeenCalledWith('card_1')
    fireEvent.click(screen.getByRole('button', { name: 'Done' }))
    await waitFor(() => expect(finished).toHaveBeenCalledWith('card_1'))
  })

  it('stacks several into one strip that opens', async () => {
    pending = [reminder('card_1', 'Review the PR'), reminder('card_2', 'Chase the deploy')]
    render(<RemindersBanner />)
    await waitFor(() => screen.getByText('Review the PR'))
    expect(screen.queryByText('Chase the deploy')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: '1 more' }))
    expect(screen.getByText('Chase the deploy')).toBeTruthy()
  })

  it('is read again when one goes off, and the clock rearmed when the window comes back', async () => {
    render(<RemindersBanner />)
    await act(async () => undefined)
    pending = [reminder('card_3', 'Call the accountant')]
    await act(async () => heard['reminder:fired']!({ reminders: pending }))
    await waitFor(() => screen.getByText('Call the accountant'))
    window.dispatchEvent(new Event('focus'))
    expect(rearmed).toHaveBeenCalled()
  })
})

describe('putting a reminder off', () => {
  it('offers a quarter of an hour, an hour, and tomorrow morning', () => {
    const now = new Date(2026, 9, 1, 18, 30).getTime() / 1000
    const [quarter, hour, tomorrow] = laterOptions(now)
    expect(quarter!.until - now).toBe(900)
    expect(hour!.until - now).toBe(3600)
    const morning = new Date(tomorrow!.until * 1000)
    expect([morning.getDate(), morning.getHours(), morning.getMinutes()]).toEqual([2, 9, 0])
  })
})
