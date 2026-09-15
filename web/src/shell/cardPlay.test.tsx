import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Played, Step } from '../gen/bindings'
import { CardPlay } from './CardPlay'
import { CardWork } from './CardWork'

afterEach(cleanup)

let answer: Played = { run: null, needsConfirming: false, laneRunsNothing: false }
let refusal: string | null = null
const played = vi.fn()

vi.mock('./live', () => ({
  ask: (call: () => unknown) =>
    Promise.resolve(
      refusal
        ? { data: null, error: refusal, loading: false }
        : { data: call(), error: null, loading: false },
    ),
  commands: {
    agentsKnown: () => [],
    appsList: () => [],
    cardPlay: (_project: string, card: string, confirmed: boolean) => {
      played(card, confirmed)
      return answer
    },
  },
}))

vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'p1' }, show: vi.fn() }) }))

const step = (over: Partial<Step> = {}): Step => ({
  id: 'step_1',
  kind: 'command',
  name: 'the tests',
  config: '{}',
  irreversible: false,
  ...over,
})

beforeEach(() => {
  answer = { run: null, needsConfirming: false, laneRunsNothing: false }
  refusal = null
  played.mockClear()
})

describe('playing a card', () => {
  /* One meaning, and only one. A button that does one thing when the lane has
     a step and something else when it does not is a button with two invisible
     meanings. */
  it('names the step it will run', () => {
    render(<CardPlay cardId="card_1" step={step()} onPlayed={vi.fn()} />)
    expect(screen.getByText('Run the tests')).toBeTruthy()
  })

  it('says the lane runs nothing rather than pretending to play', () => {
    render(<CardPlay cardId="card_1" step={null} onPlayed={vi.fn()} />)
    expect(screen.getByText('This lane runs nothing on its own.')).toBeTruthy()
    expect(screen.queryByRole('button')).toBeNull()
  })

  it('starts the run unconfirmed, for an ordinary step', async () => {
    const onPlayed = vi.fn()
    render(<CardPlay cardId="card_1" step={step()} onPlayed={onPlayed} />)
    fireEvent.click(screen.getByText('Run the tests'))
    await waitFor(() => expect(played).toHaveBeenCalledWith('card_1', false))
    await waitFor(() => expect(onPlayed).toHaveBeenCalled())
  })

  /* `card.move` uses the same word for the same reason: a deploy is not fired
     by a click somebody might not have meant. */
  it('asks before a step with no undo, and says so on the button', async () => {
    answer = { run: null, needsConfirming: true, laneRunsNothing: false }
    const onPlayed = vi.fn()
    render(
      <CardPlay
        cardId="card_1"
        step={step({ name: 'deploy', irreversible: true })}
        onPlayed={onPlayed}
       
      />,
    )
    expect(screen.getByText('no undo')).toBeTruthy()

    fireEvent.click(screen.getByText('Run deploy'))
    expect(await screen.findByText('Run deploy?')).toBeTruthy()
    // Nothing happened yet — the first press only asked.
    expect(onPlayed).not.toHaveBeenCalled()
  })

  it('runs it once the question is answered', async () => {
    answer = { run: null, needsConfirming: true, laneRunsNothing: false }
    render(
      <CardPlay
        cardId="card_1"
        step={step({ name: 'deploy', irreversible: true })}
        onPlayed={vi.fn()}
       
      />,
    )
    fireEvent.click(screen.getByText('Run deploy'))
    await screen.findByText('Run deploy?')

    answer = { run: null, needsConfirming: false, laneRunsNothing: false }
    fireEvent.click(screen.getAllByText('Run deploy').at(-1)!)
    await waitFor(() => expect(played).toHaveBeenLastCalledWith('card_1', true))
  })

  /* A second run on one card is two processes writing one checkout, and the
     backend refuses it. The refusal is what the screen says. */
  it('shows the refusal rather than swallowing it', async () => {
    refusal = 'something is already running on this card'
    render(<CardPlay cardId="card_1" step={step()} onPlayed={vi.fn()} />)
    fireEvent.click(screen.getByText('Run the tests'))
    expect(await screen.findByText(/already running/)).toBeTruthy()
  })
})

describe('the work section of a card', () => {
  const work = (step: Step | null) =>
    render(
      <CardWork
        cardId="card_1"
        worktree={null}
        runs={[]}
        onChanged={vi.fn()}
        play={<CardPlay cardId="card_1" step={step} onPlayed={vi.fn()} />}
      />,
    )

  it('offers one terminal when the lane runs nothing', () => {
    work(null)
    expect(screen.getAllByRole('button', { name: 'Open a terminal' })).toHaveLength(1)
  })

  it('puts play, the checkout and the terminal under one heading', () => {
    work(step())
    expect(screen.getAllByRole('heading').map((heading) => heading.textContent)).toEqual(['Work'])
    const section = screen.getByRole('heading', { name: 'Work' }).closest('section')
    expect(section?.contains(screen.getByRole('button', { name: /Run the tests/ }))).toBe(true)
    expect(section?.contains(screen.getByRole('button', { name: 'Make a checkout' }))).toBe(true)
    expect(section?.contains(screen.getByRole('button', { name: 'Open a terminal' }))).toBe(true)
  })
})

