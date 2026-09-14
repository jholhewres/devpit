import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { PaneCost, Usage } from '../gen/bindings'
import { Monitor } from './Monitor'

afterEach(cleanup)

const pane = (over: Partial<PaneCost> = {}): PaneCost => ({
  paneId: 'leaf_1',
  label: 'Claude Code',
  agent: 'claude',
  memoryKb: 1_085_440,
  cpuTenths: 53,
  processes: 3,
  ...over,
})

const show = (over: Partial<Usage> = {}) => {
  const closed = vi.fn()
  const shown = vi.fn()
  const view = render(
    <Monitor
      usage={{
        memoryKb: 1_085_440,
        cpuTenths: 53,
        proportional: true,
        panes: [pane()],
        ...over,
      }}
      onClose={closed}
      onShow={shown}
    />,
  )
  return { view, closed, shown }
}

/*
 * By pane rather than by process: `node` twice over says nothing, and "Claude
 * Code, three processes, 1.06 GB" says what to close.
 */

describe('what each terminal costs', () => {
  it('names the agent, not the executable', () => {
    show()
    expect(screen.getByText('Claude Code')).toBeTruthy()
  })

  it('counts the whole tree, and says how many processes it found', () => {
    // An agent's cost is mostly its children. A number that leaves them out
    // is a number saying an agent is free.
    show()
    expect(screen.getByText('3 processes')).toBeTruthy()
  })

  it('says one process in the singular', () => {
    show({ panes: [pane({ processes: 1 })] })
    expect(screen.getByText('1 process')).toBeTruthy()
  })

  it('opens the pane it is about', () => {
    const { shown } = show()
    fireEvent.click(screen.getByText('Claude Code'))
    expect(shown).toHaveBeenCalledWith('leaf_1')
  })

  it('leaves out a pane costing nothing, which is a shell at a prompt', () => {
    show({ panes: [pane(), pane({ paneId: 'leaf_2', label: 'zsh', memoryKb: 0 })] })
    expect(screen.queryByText('zsh')).toBeNull()
  })

  it('says so when there is nothing running', () => {
    show({ panes: [] })
    expect(screen.getByText(/Nothing is running/)).toBeTruthy()
  })
})

describe('a total that might be wrong says so', () => {
  it('marks a total that counted shared memory more than once', () => {
    // Resident overstates a process tree — 44% on the machine this was
    // measured on. A number that might be half wrong arrives labelled.
    const { view } = show({ proportional: false })
    expect(view.container.querySelector('.hud__warn')).toBeTruthy()
    expect(screen.getByTitle(/counted more than once/)).toBeTruthy()
  })

  it('marks nothing when the shared pages were divided', () => {
    const { view } = show({ proportional: true })
    expect(view.container.querySelector('.hud__warn')).toBeNull()
    expect(screen.getByTitle(/really in use/)).toBeTruthy()
  })
})
