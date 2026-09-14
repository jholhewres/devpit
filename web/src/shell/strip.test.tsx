import { cleanup, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Usage } from '../gen/bindings'
import { StatusStrip } from './StatusStrip'

afterEach(cleanup)

let usage: Usage = { memoryKb: 0, cpuTenths: 0, proportional: true, panes: [] }
let project: { id: string } | null = { id: 'p' }

vi.mock('./useShell', () => ({ useShell: () => ({ project, focus: vi.fn() }) }))
vi.mock('./useUsage', () => ({ useUsage: () => usage }))

beforeEach(() => {
  usage = { memoryKb: 0, cpuTenths: 0, proportional: true, panes: [] }
  project = { id: 'p' }
})

/*
 * The strip hid itself whenever nothing was measured, which made "nothing is
 * running" and "this is broken" the same picture: somebody opened the window,
 * looked for it, and found nothing to tell them which.
 */

describe('a strip you can find', () => {
  it('is there with a project open and nothing running', () => {
    render(<StatusStrip />)
    expect(screen.getByText('No terminals')).toBeTruthy()
  })

  it('shows no figures when there is nothing to measure', () => {
    // `0% · 0 MB` is a line the eye learns to skip, and then misses when it
    // matters. Going quiet is not the same as going away.
    const { container } = render(<StatusStrip />)
    expect(container.querySelector('.strip__n')).toBeNull()
  })

  it('says nothing at all with no project open', () => {
    project = null
    const { container } = render(<StatusStrip />)
    expect(container.querySelector('.strip')).toBeNull()
  })
})

describe('once something is running', () => {
  beforeEach(() => {
    usage = {
      memoryKb: 1_085_440,
      cpuTenths: 53,
      proportional: true,
      panes: [
        {
          paneId: 'leaf_1',
          label: 'Claude Code',
          agent: 'claude',
          memoryKb: 1_085_440,
          cpuTenths: 53,
          processes: 3,
        },
      ],
    }
  })

  it('puts the figures beside the count', async () => {
    render(<StatusStrip />)
    await waitFor(() => expect(screen.getByText('1 terminal')).toBeTruthy())
    expect(screen.getByText('1.04 GB')).toBeTruthy()
    expect(screen.getByText('5.3%')).toBeTruthy()
  })

  it('counts more than one in the plural', () => {
    usage = { ...usage, panes: [...usage.panes, { ...usage.panes[0]!, paneId: 'leaf_2' }] }
    render(<StatusStrip />)
    expect(screen.getByText('2 terminals')).toBeTruthy()
  })

  it('marks a total that counted shared memory twice', () => {
    // Resident overstates a process tree by 44% on the machine this was
    // measured on. A number that might be half wrong arrives labelled.
    usage = { ...usage, proportional: false }
    const { container } = render(<StatusStrip />)
    expect(container.querySelector('.strip__warn')).toBeTruthy()
  })
})
