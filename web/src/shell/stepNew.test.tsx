import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Step } from '../gen/bindings'
import { StepNew } from './StepNew'

afterEach(cleanup)

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: { agentProfiles: () => [], agentsList: () => ({ agents: [] }) },
}))

describe('the form that edits a step', () => {
  it('saves over what is stored, not instead of it', () => {
    const step: Step = {
      id: 'step_1',
      kind: 'command',
      name: 'tests',
      config: JSON.stringify({ command: 'make test', needsWorktree: true }),
      irreversible: false,
    }
    const onDone = vi.fn()
    render(<StepNew step={step} onDone={onDone} onCancel={vi.fn()} />)
    fireEvent.change(screen.getByPlaceholderText('make test'), { target: { value: 'make check' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(JSON.parse(onDone.mock.calls[0]![2] as string)).toEqual({ command: 'make check', needsWorktree: true })
  })
})
