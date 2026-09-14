import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Part, Project } from '../gen/bindings'
import { Acts } from './Acts'

afterEach(cleanup)

const show = vi.fn()
const project = { id: 'p', name: 'demo', rootPath: '/work/demo' } as unknown as Project
vi.mock('./useShell', () => ({ useShell: () => ({ project, show }) }))

const edit = (input: object, output = 'ok', isError = false): Part[] => [
  { kind: 'tool_call', id: 'e1', name: 'Edit', input: JSON.stringify(input), state: 'ok', parent: null },
  { kind: 'tool_result', call_id: 'e1', output, is_error: isError, parent: null },
]

describe('an edit in the thread', () => {
  it('shows the diff without being opened, with what it added and removed', () => {
    render(<Acts parts={edit({ file_path: '/work/demo/notes.txt', old_string: 'alpha', new_string: 'beta' })} live />)
    expect(screen.getByText('+1')).toBeTruthy()
    expect(screen.getByText('−1')).toBeTruthy()
    expect(screen.getByText('beta')).toBeTruthy()
  })

  it('opens a file inside the project in a tab, by its project path', () => {
    render(<Acts parts={edit({ file_path: '/work/demo/src/a.rs', old_string: 'a', new_string: 'b' })} live />)
    fireEvent.click(screen.getByRole('button', { name: 'src/a.rs' }))
    expect(show).toHaveBeenCalledWith('file', { id: 'file:src/a.rs', path: 'src/a.rs' })
  })

  it('shows a path outside the project without offering to open it', () => {
    render(<Acts parts={edit({ file_path: '/etc/hosts', old_string: 'a', new_string: 'b' })} live />)
    expect(screen.queryByRole('button', { name: '/etc/hosts' })).toBeNull()
    expect(screen.getByText('/etc/hosts')).toBeTruthy()
  })

  it('shows a write of a new file as all additions', () => {
    const write: Part[] = [
      { kind: 'tool_call', id: 'w1', name: 'Write', input: JSON.stringify({ file_path: '/work/demo/new.md', content: 'one\ntwo\n' }), state: 'ok', parent: null },
      { kind: 'tool_result', call_id: 'w1', output: 'ok', is_error: false, parent: null },
    ]
    render(<Acts parts={write} live />)
    expect(screen.getByText('+2')).toBeTruthy()
    expect(screen.getByText('−0')).toBeTruthy()
  })

  it('shows the error of a failed edit instead of a diff that never happened', () => {
    render(
      <Acts
        parts={edit({ file_path: '/work/demo/a.rs', old_string: 'missing', new_string: 'x' }, 'String to replace not found in file.', true)}
        live
      />,
    )
    expect(screen.getByText('String to replace not found in file.')).toBeTruthy()
    expect(screen.queryByText('+1')).toBeNull()
  })

  it('colours the lines with the lexer the editor uses', () => {
    const { container } = render(
      <Acts parts={edit({ file_path: '/work/demo/src/main.rs', old_string: 'fn old() {}', new_string: 'fn main() {}' })} live />,
    )
    // `fn` is a keyword in Rust; the same token class the editor paints.
    expect(container.querySelector('.ecard .tok--word')).toBeTruthy()
  })
})

