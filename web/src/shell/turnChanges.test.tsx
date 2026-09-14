import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Part } from '../gen/bindings'
import { byAgent, totals } from './turnChanges'
import { TurnChanges } from './TurnChanges'
import { Turn } from './Turn'

afterEach(cleanup)

const show = vi.fn()
vi.mock('./useShell', () => ({ useShell: () => ({ show }) }))

const editOf = (path: string): Part => ({
  kind: 'tool_call',
  id: path,
  name: 'Edit',
  input: JSON.stringify({ file_path: path, old_string: 'a', new_string: 'b' }),
  state: 'ok',
  parent: null,
})

const files = [
  { path: 'src/main.rs', added: 3, removed: 1 },
  { path: 'Cargo.lock', added: 40, removed: 2 },
]

describe('which changes the agent made itself', () => {
  it('matches an absolute edit path to the relative changed path', () => {
    expect(byAgent(files[0]!, [editOf('/work/demo/src/main.rs')])).toBe(true)
  })

  it('does not match a file that only ends with the same name', () => {
    // `xmain.rs` is not `main.rs`: the match is on a path boundary.
    expect(byAgent({ path: 'main.rs', added: 1, removed: 0 }, [editOf('/work/demo/src/xmain.rs')])).toBe(false)
  })

  it('adds the counts up', () => {
    expect(totals(files)).toEqual({ added: 43, removed: 3 })
  })
})

describe('the rollup under a turn', () => {
  it('lists every changed file with the totals', () => {
    render(<TurnChanges files={files} parts={[editOf('/work/demo/src/main.rs')]} />)
    expect(screen.getByText('2 files changed')).toBeTruthy()
    expect(screen.getByText('+43')).toBeTruthy()
  })

  it('marks a file no edit call names as changed by a command', () => {
    render(<TurnChanges files={files} parts={[editOf('/work/demo/src/main.rs')]} />)
    // Cargo.lock changed because the agent ran cargo, not because it edited it.
    expect(screen.getAllByText('by a command')).toHaveLength(1)
  })

  it('opens the diff of a file', () => {
    render(<TurnChanges files={files} parts={[]} />)
    fireEvent.click(screen.getByText('src/main.rs'))
    expect(show).toHaveBeenCalledWith('diff', { id: 'diff:src/main.rs', path: 'src/main.rs', title: 'main.rs diff' })
  })

  it('draws nothing for a turn that changed nothing', () => {
    const { container } = render(<TurnChanges files={[]} parts={[]} />)
    expect(container.firstChild).toBeNull()
  })
})

describe('a finished turn', () => {
  const message = (streaming: boolean) =>
    ({
      id: 'm',
      turnId: 't',
      role: 'assistant',
      createdAt: 0,
      streaming,
      parts: [
        { kind: 'text', text: 'Done.', parent: null },
        { kind: 'changes', files: [{ path: 'src/main.rs', added: 2, removed: 0 }] },
      ],
    }) as unknown as Parameters<typeof Turn>[0]['message']

  it('shows what it changed under it', () => {
    render(<Turn message={message(false)} />)
    expect(screen.getByText('1 file changed')).toBeTruthy()
  })

  it('waits until the turn is over, when the list is final', () => {
    render(<Turn message={message(true)} />)
    expect(screen.queryByText('1 file changed')).toBeNull()
  })
})

