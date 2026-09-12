import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Played, Step } from '../gen/bindings'
import { CardPlay } from './CardPlay'

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
    cardPlay: (_project: string, card: string, confirmed: boolean) => {
      played(card, confirmed)
      return answer
    },
  },
}))

vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'p1' } }) }))

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
    render(<CardPlay cardId="card_1" step={step()} onPlayed={vi.fn()} onOpenTerminal={vi.fn()} />)
    expect(screen.getByText('Run the tests')).toBeTruthy()
  })

  it('says the lane runs nothing rather than pretending to play', () => {
    const onOpenTerminal = vi.fn()
    render(<CardPlay cardId="card_1" step={null} onPlayed={vi.fn()} onOpenTerminal={onOpenTerminal} />)
    expect(screen.getByText(/runs nothing on its own/)).toBeTruthy()
    fireEvent.click(screen.getByText('Open a terminal'))
    expect(onOpenTerminal).toHaveBeenCalled()
  })

  it('starts the run unconfirmed, for an ordinary step', async () => {
    const onPlayed = vi.fn()
    render(<CardPlay cardId="card_1" step={step()} onPlayed={onPlayed} onOpenTerminal={vi.fn()} />)
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
        onOpenTerminal={vi.fn()}
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
        onOpenTerminal={vi.fn()}
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
    render(<CardPlay cardId="card_1" step={step()} onPlayed={vi.fn()} onOpenTerminal={vi.fn()} />)
    fireEvent.click(screen.getByText('Run the tests'))
    expect(await screen.findByText(/already running/)).toBeTruthy()
  })
})
