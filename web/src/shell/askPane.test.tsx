import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { ProjectAgentAnswer } from '../gen/bindings'
import { AskPane } from './AskPane'
import { ProjectPlaces } from './ProjectPlaces'

const called = vi.fn()
const shown = vi.fn()
let answer: ProjectAgentAnswer = {
  answer: 'Tests run with `make test`, defined in Makefile.',
  woke: true,
  cited: ['Makefile'],
}
let here: { id: string; name: string; orchestrator: string | null } = { id: 'prj_api', name: 'api', orchestrator: null }

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: new Proxy(
    {},
    {
      get: (_, name) =>
        typeof name !== 'string' || name === 'then'
          ? undefined
          : async (...args: unknown[]) => {
              called(name, ...args)
              return name === 'projectAgentChat' ? answer : { proposals: [], decisions: [] }
            },
    },
  ),
}))
vi.mock('./useShell', () => ({ useShell: () => ({ project: here }) }))
vi.mock('./shellStore', () => ({ useShellPick: () => shown }))
vi.mock('./MarkdownView', () => ({ Markdown: ({ source }: { source: string }) => <p>{source}</p> }))

afterEach(() => {
  cleanup()
  called.mockReset()
  shown.mockReset()
  here = { id: 'prj_api', name: 'api', orchestrator: null }
})

const asked = async (question: string): Promise<void> => {
  fireEvent.change(screen.getByRole('textbox', { name: 'Ask the project' }), { target: { value: question } })
  fireEvent.keyDown(screen.getByRole('textbox', { name: 'Ask the project' }), { key: 'Enter' })
  await screen.findByText(answer.answer)
}

describe('Ask the project', () => {
  it('is a place of every project, and not of an orchestrator', () => {
    const open = vi.fn()
    render(<ProjectPlaces open={open} showing={() => false} />)
    fireEvent.click(screen.getByRole('button', { name: 'Ask the project' }))
    expect(open).toHaveBeenCalledWith('ask')
    cleanup()
    here = { id: 'orch', name: 'Jhol', orchestrator: 'claude' }
    render(<ProjectPlaces open={open} showing={() => false} />)
    expect(screen.queryByRole('button', { name: 'Ask the project' })).toBeNull()
  })

  /* Each test asks in a project of its own: the pane keeps each project's turns. */
  it('asks the project agent and shows the answer with the files it cites', async () => {
    render(<AskPane />)
    await asked('How are tests run?')
    expect(called).toHaveBeenCalledWith('projectAgentChat', 'prj_api', 'How are tests run?')
    fireEvent.click(screen.getByRole('button', { name: 'Makefile' }))
    expect(shown).toHaveBeenCalledWith('file', { id: 'file:Makefile', path: 'Makefile' })
  })

  it('turns an answer into a fact only as a proposal waiting for approval', async () => {
    here = { id: 'prj_web', name: 'web', orchestrator: null }
    render(<AskPane />)
    await asked('How are tests run?')
    fireEvent.click(screen.getByRole('button', { name: 'Save as fact' }))
    const fact = screen.getByRole('textbox', { name: 'Fact' })
    expect(fact).toHaveProperty('value', 'Tests run with `make test`, defined in Makefile.')
    fireEvent.change(fact, { target: { value: 'Tests run with make test.' } })
    fireEvent.click(screen.getByRole('button', { name: 'Propose' }))
    await screen.findByText(/waits for your approval/)
    expect(called).toHaveBeenCalledWith('projectAgentFact', 'prj_web', 'Tests run with make test.', ['Makefile'])
    expect(called.mock.calls.map(([name]) => name)).toEqual(['projectAgentChat', 'projectAgentFact'])
  })

  it('offers no fact from an answer that cites no file', async () => {
    answer = { answer: 'I could not tell.', woke: false, cited: [] }
    here = { id: 'prj_cli', name: 'cli', orchestrator: null }
    render(<AskPane />)
    await asked('Who wrote this?')
    expect(screen.queryByRole('button', { name: 'Save as fact' })).toBeNull()
  })
})
