import { cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Schedule, Schedules } from '../gen/bindings'

/* Friday 2026-11-13, 12:00 UTC: 09:00 in São Paulo. */
const AT = 1_794_571_200

const routine: Schedule = {
  id: 'routine_1',
  kind: 'routine',
  title: 'Morning summary',
  at: AT,
  projectId: 'orch',
  project: 'Jhol',
  cardNumber: null,
  routine: { kind: 'morning', minute: 540, days: [1, 2, 3, 4, 5], zone: 'America/Sao_Paulo', offsetMinutes: -180 },
  state: 'waiting',
  lastRun: null,
}
const reminder: Schedule = {
  id: 'card_9',
  kind: 'reminder',
  title: 'Review the PR',
  at: AT + 3600,
  projectId: 'prj_api',
  project: 'api',
  cardNumber: 42,
  routine: null,
  state: 'due',
  lastRun: AT + 3600,
}
const listed: Schedules = {
  schedules: [routine, reminder],
  proposed: [{ id: 'proposal_1', routine: { ...routine.routine!, kind: 'ci', minute: 510 }, reminder: null, why: 'You asked for CI news each morning' }],
}

const called = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: new Proxy(
    {},
    {
      get: (_, name) =>
        typeof name !== 'string' || name === 'then'
          ? undefined
          : (...args: unknown[]) => {
          called(name, ...args)
          return name === 'orchestratorLinks' ? ['prj_api'] : listed
        },
    },
  ),
}))
vi.mock('./useShell', () => ({
  useShell: () => ({
    project: { id: 'orch', name: 'Jhol', orchestrator: 'claude' },
    projects: [
      { id: 'orch', name: 'Jhol', orchestrator: 'claude' },
      { id: 'prj_api', name: 'api', orchestrator: null },
      { id: 'prj_other', name: 'other', orchestrator: null },
    ],
  }),
}))
vi.mock('./window', () => ({ onCarried: () => () => {} }))

const { SchedulesPane } = await import('./SchedulesPane')

afterEach(() => {
  cleanup()
  called.mockClear()
})

const row = async (title: string): Promise<HTMLElement> => (await screen.findByText(title)).closest('[role="row"]') as HTMLElement
const calls = (name: string): unknown[][] => called.mock.calls.filter((one) => one[0] === name).map((one) => one.slice(1))

describe('the Schedules menu of an orchestrator', () => {
  it('lists each reminder and routine with its time, subject, place, repeat and state', async () => {
    render(<SchedulesPane />)
    const morning = await row('Morning summary')
    expect(within(morning).getByText(/09:00/)).toBeTruthy()
    expect(within(morning).getByText('Weekdays')).toBeTruthy()
    expect(within(morning).getByText('Waiting')).toBeTruthy()
    const pr = await row('Review the PR')
    expect(within(pr).getByText('api · #42')).toBeTruthy()
    expect(within(pr).getByText('Once')).toBeTruthy()
    expect(within(pr).getByText('Went off')).toBeTruthy()
  })

  it('pauses, edits and deletes a routine from its row', async () => {
    render(<SchedulesPane />)
    const morning = await row('Morning summary')
    fireEvent.click(within(morning).getByText('Pause'))
    expect(calls('scheduleRoutinePause')).toEqual([['orch', 'routine_1', true]])
    fireEvent.click(within(morning).getByText('Edit'))
    fireEvent.change(screen.getByLabelText('At'), { target: { value: '07:30' } })
    fireEvent.click(screen.getByText('Save routine'))
    expect(calls('scheduleRoutineSave')[0]).toEqual(['orch', 'routine_1', expect.objectContaining({ kind: 'morning', minute: 450, days: [1, 2, 3, 4, 5] })])
    fireEvent.click(within(morning).getByText('Delete'))
    expect(calls('scheduleRoutineDelete')).toEqual([['orch', 'routine_1']])
  })

  it('resolves, moves and deletes a reminder from its row', async () => {
    render(<SchedulesPane />)
    const pr = await row('Review the PR')
    fireEvent.click(within(pr).getByText('Resolve'))
    expect(calls('reminderDone')).toEqual([['card_9']])
    fireEvent.click(within(pr).getByText('Edit'))
    fireEvent.change(screen.getByLabelText('New time for Review the PR'), { target: { value: '2026-11-20T10:00' } })
    fireEvent.click(screen.getByText('Save'))
    const moved = calls('cardSetDue')[0]
    expect(moved.slice(0, 2)).toEqual(['prj_api', 'card_9'])
    expect(moved[2]).toBe(Math.floor(new Date('2026-11-20T10:00').getTime() / 1000))
    fireEvent.click(within(pr).getByText('Delete'))
    expect(calls('cardSetDue')[1]).toEqual(['prj_api', 'card_9', null, false])
  })

  it('makes a routine and a reminder by hand, on the orchestrator or a linked project', async () => {
    render(<SchedulesPane />)
    fireEvent.click(await screen.findByText('New routine'))
    fireEvent.change(screen.getByLabelText('Routine'), { target: { value: 'review' } })
    fireEvent.click(screen.getByText('Save routine'))
    expect(calls('scheduleRoutineSave')[0]).toEqual(['orch', null, expect.objectContaining({ kind: 'review', minute: 540 })])

    fireEvent.click(screen.getByText('New reminder'))
    const on = screen.getByLabelText('On') as HTMLSelectElement
    expect([...on.options].map((one) => one.textContent)).toEqual(['Jhol', 'api'])
    fireEvent.change(screen.getByLabelText('Remind me of'), { target: { value: 'Pay the invoice' } })
    fireEvent.change(on, { target: { value: 'prj_api' } })
    fireEvent.click(screen.getByText('Save reminder'))
    expect(calls('scheduleRemind')[0].slice(0, 3)).toEqual(['orch', 'prj_api', 'Pay the invoice'])
  })

  it('adds what the orchestrator proposed only on the person’s click', async () => {
    render(<SchedulesPane />)
    const proposal = await screen.findByRole('group', { name: 'Proposed by the orchestrator' })
    expect(within(proposal).getByText('Proposed: CI failures at 08:30, weekdays')).toBeTruthy()
    expect(calls('scheduleProposal')).toEqual([])
    fireEvent.click(within(proposal).getByText('Add'))
    expect(calls('scheduleProposal')[0].slice(0, 3)).toEqual(['orch', 'proposal_1', true])
  })
})
