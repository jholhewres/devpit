import { act, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { Project, SessionLayout } from '../gen/bindings'
import { TerminalStage } from './TerminalStage'

const { ensure } = vi.hoisted(() => ({ ensure: vi.fn() }))

vi.mock('../gen/bindings', async (original) => {
  const module = await original<typeof import('../gen/bindings')>()
  return {
    ...module,
    commands: {
      ...module.commands,
      sessionEnsure: ensure,
      sessionFocus: vi.fn()
    }
  }
})

vi.mock('../session/LayoutView', () => ({
  LayoutView: () => <div data-testid="live-layout">live</div>
}))

const project: Project = {
  id: 'project',
  name: 'Project',
  rootPath: '/project',
  group: null,
  accent: '#fff',
  worktrees: [],
  unreadable: null
}

const layout: SessionLayout = {
  projectId: 'project',
  focusedId: 'leaf_a',
  tree: {
    type: 'leaf',
    id: 'leaf_a',
    tmuxTarget: 'session:leaf_a',
    kind: 'terminal',
    agent: 'none'
  }
}

describe('TerminalStage', () => {
  it('waits for tmux recovery before attaching persisted leaves', async () => {
    let resolve!: (answer: { status: 'ok'; data: SessionLayout }) => void
    ensure.mockReturnValueOnce(
      new Promise<{ status: 'ok'; data: SessionLayout }>((done) => {
        resolve = done
      })
    )

    render(
      <TerminalStage
        project={project}
        worktree={null}
        layout={layout}
        onLayout={() => undefined}
        tmux
      />
    )

    expect(screen.queryByTestId('live-layout')).toBeNull()
    expect(screen.getByText('Attaching session…')).toBeTruthy()

    await act(async () => resolve({ status: 'ok', data: layout }))
    expect(screen.getByTestId('live-layout')).toBeTruthy()
  })

  it('states why a session cannot start without tmux', () => {
    render(
      <TerminalStage
        project={project}
        worktree={null}
        layout={null}
        onLayout={() => undefined}
        tmux={false}
      />
    )
    expect(screen.getByText(/tmux is not installed/)).toBeTruthy()
  })
})
