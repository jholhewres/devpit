import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Step } from '../gen/bindings'
import { LaneStep } from './LaneStep'

afterEach(cleanup)

const step = (over: Partial<Step> = {}): Step => ({
  id: 'step_1',
  kind: 'command',
  name: 'tests',
  config: '{}',
  irreversible: false,
  ...over,
})

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: { agentProfiles: () => [{ id: 'prof_glm', label: 'glm' }] },
}))

const lanes = [
  { id: 'col_review', name: 'review' },
  { id: 'col_ship', name: 'ship' },
]

const draw = (over: Partial<Parameters<typeof LaneStep>[0]> = {}) => {
  const onFlow = vi.fn()
  render(
    <LaneStep
      step={step()}
      steps={[step()]}
      lanes={lanes}
      onPass={null}
      autonomy="manual"
      onPick={vi.fn()}
      onCreate={vi.fn()}
      onFlow={onFlow}
      {...over}
    />,
  )
  return onFlow
}

describe('what a lane does when its step passes', () => {
  it('offers nothing to decide until the lane runs something', () => {
    draw({ step: null })
    fireEvent.click(screen.getByTitle(/runs nothing/i))
    expect(screen.queryByText('When it passes')).toBeNull()
  })

  it('starts with the card staying put', () => {
    draw()
    fireEvent.click(screen.getByTitle('Runs tests'))
    expect(screen.getByText('Stay here').getAttribute('aria-checked')).toBe('true')
  })

  /* The default is manual and it has to survive being given a destination:
     choosing where a pass goes is not the same as agreeing that the product
     may move the card there by itself. */
  it('asks before moving, the first time a destination is chosen', () => {
    const onFlow = draw()
    fireEvent.click(screen.getByTitle('Runs tests'))
    fireEvent.click(screen.getByText('Move to review'))
    expect(onFlow).toHaveBeenCalledWith('col_review', 'ask')
  })

  it('keeps an autonomy already chosen when the destination changes', () => {
    const onFlow = draw({ onPass: 'col_review', autonomy: 'auto' })
    fireEvent.click(screen.getByTitle('Runs tests'))
    fireEvent.click(screen.getByText('Move to ship'))
    expect(onFlow).toHaveBeenCalledWith('col_ship', 'auto')
  })

  it('does not offer to send a card to the lane it is already in', () => {
    draw({ onPass: 'col_review' })
    fireEvent.click(screen.getByTitle('Runs tests'))
    // `lanes` is the other lanes; this one is never among them.
    expect(screen.queryByText('Move to tests')).toBeNull()
  })

  it('has nothing to say about how it moves until there is somewhere to move', () => {
    draw()
    fireEvent.click(screen.getByTitle('Runs tests'))
    expect(screen.queryByText('And moves it')).toBeNull()
  })

  it('offers both ways once there is', () => {
    const onFlow = draw({ onPass: 'col_review', autonomy: 'ask' })
    fireEvent.click(screen.getByTitle('Runs tests'))
    /* The label sits in a span because the other option carries a badge
       beside it, so the state is on the button above it. */
    expect(
      screen.getByText('When you say so').closest('button')?.getAttribute('aria-checked'),
    ).toBe('true')
    fireEvent.click(screen.getByText('On its own'))
    expect(onFlow).toHaveBeenCalledWith('col_review', 'auto')
  })

  it('clearing the destination puts the lane back to manual', () => {
    const onFlow = draw({ onPass: 'col_review', autonomy: 'auto' })
    fireEvent.click(screen.getByTitle('Runs tests'))
    fireEvent.click(screen.getByText('Stay here'))
    expect(onFlow).toHaveBeenCalledWith(null, 'manual')
  })

  /* A lane that moves cards on its own should look different from one that
     does not, without opening its menu. */
  it('says on the lane when it moves cards by itself', () => {
    const { container } = render(
      <LaneStep
        step={step()}
        steps={[step()]}
        lanes={lanes}
        onPass="col_review"
        autonomy="auto"
        onPick={vi.fn()}
        onCreate={vi.fn()}
        onFlow={vi.fn()}
      />,
    )
    expect(container.querySelector('.lstep__auto')).toBeTruthy()
  })

  it('says nothing extra when it does not', () => {
    const { container } = render(
      <LaneStep
        step={step()}
        steps={[step()]}
        lanes={lanes}
        onPass="col_review"
        autonomy="ask"
        onPick={vi.fn()}
        onCreate={vi.fn()}
        onFlow={vi.fn()}
      />,
    )
    expect(container.querySelector('.lstep__auto')).toBeNull()
  })
})

describe('the form that makes a step', () => {
  const open = (over: Partial<Parameters<typeof LaneStep>[0]> = {}) => {
    draw(over)
    fireEvent.click(screen.getByTitle('Runs tests'))
    fireEvent.click(screen.getByText(/New step/))
  }

  /* What this covers: the form used to save the one line the person typed,
     which is not JSON, which is not what any runner reads. */
  it('saves a command as the JSON its runner reads', () => {
    const onCreate = vi.fn()
    open({ onCreate })
    fireEvent.change(screen.getByPlaceholderText('tests'), { target: { value: 'suite' } })
    fireEvent.change(screen.getByPlaceholderText('make test'), { target: { value: 'make test' } })
    fireEvent.change(screen.getByPlaceholderText('as long as it takes'), { target: { value: '600' } })
    fireEvent.click(screen.getByText('Create'))
    expect(onCreate).toHaveBeenCalledWith('command', 'suite', '{"command":"make test","timeoutSeconds":600}', false)
  })

  it('asks a session which account runs it, and not for a command', async () => {
    open()
    fireEvent.click(screen.getByText('Session'))
    expect(screen.queryByPlaceholderText('make test')).toBeNull()
    expect(await screen.findByText('glm')).toBeTruthy()
  })

  it('keeps Create out of reach until the kind has what it needs', () => {
    open()
    fireEvent.change(screen.getByPlaceholderText('tests'), { target: { value: 'suite' } })
    expect(screen.getByText('Create').hasAttribute('disabled')).toBe(true)
  })
})
