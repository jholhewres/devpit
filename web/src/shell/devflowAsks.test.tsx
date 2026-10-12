import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { DevflowAsk, DevflowAsks as Asks } from '../gen/bindings'
import { DevflowAsks } from './DevflowAsks'

const heard: Record<string, (payload: unknown) => void> = {}
vi.mock('./window', () => ({
  onCarried: (name: string, then: (payload: unknown) => void) => {
    heard[name] = then
    return () => delete heard[name]
  },
}))

const asked = (id: string, change: DevflowAsk['change']): DevflowAsk => ({
  id,
  projectId: 'prj_api',
  project: 'asc-api',
  change,
  askedAt: 1,
  detail: null,
})

let waiting: DevflowAsk[] = []
let refusal: string | null = null
const answered = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => {
    try {
      return { data: await call(), error: null, loading: false }
    } catch (why) {
      return { data: null, error: String(why), loading: false }
    }
  },
  commands: {
    devflowAsks: async (): Promise<Asks> => ({ asks: waiting }),
    devflowAnswer: async (id: string, yes: boolean): Promise<Asks> => {
      answered(id, yes)
      if (refusal) throw refusal
      waiting = waiting.filter((one) => one.id !== id)
      return { asks: waiting }
    },
  },
}))

afterEach(() => {
  cleanup()
  waiting = []
  refusal = null
  answered.mockReset()
})

describe('DevflowAsks', () => {
  it('draws nothing while nothing waits for the person', async () => {
    render(<DevflowAsks />)
    await waitFor(() => expect(screen.queryByRole('alert')).toBeNull())
  })

  it('switches Devflow on only on the click, and the card goes', async () => {
    waiting = [asked('dfa_1', 'enable')]
    render(<DevflowAsks />)
    expect(await screen.findByText('Switch Devflow on in asc-api?')).toBeTruthy()
    expect(answered).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Switch on' }))
    await waitFor(() => expect(answered).toHaveBeenCalledWith('dfa_1', true))
    await waitFor(() => expect(screen.queryByRole('alert')).toBeNull())
  })

  it('drops an ask on Not now and asks again when the app says so', async () => {
    waiting = [asked('dfa_2', 'enforce')]
    render(<DevflowAsks />)
    expect(await screen.findByText("Enforce Decisions in asc-api's Devflow?")).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Not now' }))
    await waitFor(() => expect(answered).toHaveBeenCalledWith('dfa_2', false))
    await waitFor(() => expect(screen.queryByRole('alert')).toBeNull())
    waiting = [asked('dfa_3', 'enable')]
    await act(async () => heard['devflow:asked']?.(null))
    expect(await screen.findByText('Switch Devflow on in asc-api?')).toBeTruthy()
  })

  it('keeps the card with the reason when the click could not be made', async () => {
    waiting = [asked('dfa_4', 'enable')]
    refusal = 'devflow.yaml is missing or invalid, so nothing can be switched on'
    render(<DevflowAsks />)
    fireEvent.click(await screen.findByRole('button', { name: 'Switch on' }))
    expect(await screen.findByText(/missing or invalid/)).toBeTruthy()
    expect(screen.getByText('Switch Devflow on in asc-api?')).toBeTruthy()
  })
})

describe("a team's command and an autopilot", () => {
  it('say what the click runs before it runs it', async () => {
    waiting = [
      { ...asked('dfa_5', 'command'), detail: 'acme at 0123456789ab (acme/pr): ./scripts/pr-checklist.sh' },
      { ...asked('dfa_6', 'autopilot'), detail: '[devflow] Brief: Search: Index → List' },
    ]
    render(<DevflowAsks />)
    expect(await screen.findByText("Run a team layer's gate command in asc-api?")).toBeTruthy()
    expect(screen.getByText(/\.\/scripts\/pr-checklist\.sh — code from outside the project/)).toBeTruthy()
    expect(screen.getByText('Start the autopilot in asc-api?')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Run it' }))
    await waitFor(() => expect(answered).toHaveBeenCalledWith('dfa_5', true))
  })
})
