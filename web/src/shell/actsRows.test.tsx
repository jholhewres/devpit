import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'

import type { Part } from '../gen/bindings'
import { Acts } from './Acts'

afterEach(cleanup)

const read = (id: string, file: string): Part[] => [
  { kind: 'tool_call', id, name: 'Read', input: JSON.stringify({ file_path: file }), state: 'ok', parent: null },
  { kind: 'tool_result', call_id: id, output: 'contents', is_error: false, parent: null },
]

describe('what the agent did, folded', () => {
  it('shows a run of reads as one row with the count', () => {
    render(<Acts parts={[...read('a', 'a.rs'), ...read('b', 'b.rs'), ...read('c', 'c.rs')]} live />)
    expect(screen.getByText('3 files')).toBeTruthy()
    // Folded: the files themselves are one click away, not on screen.
    expect(screen.queryByText('a.rs')).toBeNull()
  })

  it('opens to the individual calls', () => {
    render(<Acts parts={[...read('a', 'a.rs'), ...read('b', 'b.rs'), ...read('c', 'c.rs')]} live />)
    fireEvent.click(screen.getByText('3 files'))
    expect(screen.getByText('a.rs')).toBeTruthy()
    expect(screen.getByText('c.rs')).toBeTruthy()
  })

  it('leaves the call still running outside the fold', () => {
    const running: Part = { kind: 'tool_call', id: 'd', name: 'Read', input: JSON.stringify({ file_path: 'd.rs' }), state: 'running', parent: null }
    render(<Acts parts={[...read('a', 'a.rs'), ...read('b', 'b.rs'), ...read('c', 'c.rs'), running]} live />)
    expect(screen.getByText('3 files')).toBeTruthy()
    expect(screen.getAllByText('d.rs').length).toBeGreaterThan(0)
  })
})
