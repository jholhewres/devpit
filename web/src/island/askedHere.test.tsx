import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { IslandAsked, IslandSession, PendingPrompt } from '../gen/bindings'
import { AskedHere } from './AskedHere'

const onScreen: PendingPrompt = {
  question: 'Where does the viewer run?',
  options: [
    { label: 'Web, PWA later', hint: null },
    { label: 'Native app', hint: null },
  ],
  cursor: 0,
} as PendingPrompt
let shown: PendingPrompt | null = onScreen
const answered = vi.fn(async (..._args: unknown[]) => null)
vi.mock('../shell/live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    islandPrompt: async () => shown,
    islandAnswer: (...args: unknown[]) => answered(...args),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  shown = onScreen
})

const session = { sessionId: 's1' } as IslandSession
const questions: IslandAsked[] = [
  { header: 'Viewer', question: 'Where does the viewer run?', options: ['Web, PWA later', 'Native app'], multi: false },
]

describe('a question on the island', () => {
  it('says what is asked and the choices, from the hook, at once', () => {
    shown = null
    render(<AskedHere session={session} questions={questions} waiting={false} />)
    expect(screen.getByText('Where does the viewer run?')).toBeTruthy()
    expect(screen.getByText('Viewer')).toBeTruthy()
    expect(screen.getByText('Native app')).toBeTruthy()
    expect(screen.queryByRole('button')).toBeNull()
  })

  it('answers in the terminal, on the question as the screen shows it', async () => {
    render(<AskedHere session={session} questions={questions} waiting />)
    fireEvent.click(await screen.findByRole('button', { name: /Native app/ }))
    await waitFor(() => expect(answered).toHaveBeenCalledWith('s1', onScreen, 1))
  })

  it('offers no buttons when the terminal shows no question', async () => {
    shown = null
    render(<AskedHere session={session} questions={questions} waiting />)
    await waitFor(() => expect(screen.getByText('Native app')).toBeTruthy())
    expect(screen.queryByRole('button')).toBeNull()
  })
})
