import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'

import type { Part } from '../gen/bindings'
import { Acts } from './ActsRows'

afterEach(cleanup)

const read = (id: string, file: string): Part[] => [
  { kind: 'tool_call', id, name: 'Read', input: JSON.stringify({ file_path: file }), state: 'ok', parent: null },
  { kind: 'tool_result', call_id: id, output: 'contents', is_error: false, parent: null },
]
const parts = [...read('a', 'a.rs'), ...read('b', 'b.rs'), ...read('c', 'c.rs')]

describe('a fold opened in a turn that scrolls out of the window', () => {
  it('is open again when the turn is drawn again', () => {
    const first = render(<Acts parts={parts} live={false} messageId="m1" />)
    fireEvent.click(screen.getAllByRole('button')[0]!)
    fireEvent.click(screen.getByText('3 files'))
    expect(screen.getByText('a.rs')).toBeTruthy()
    first.unmount()

    render(<Acts parts={parts} live={false} messageId="m1" />)
    expect(screen.getByText('a.rs')).toBeTruthy()
  })

  it('is its own message’s: another message’s fold stays closed', () => {
    render(<Acts parts={parts} live={false} messageId="m2" />)
    expect(screen.queryByText('3 files')).toBeNull()
  })
})
