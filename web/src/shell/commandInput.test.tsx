import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

const complete = vi.fn()
vi.mock('./live', () => ({
  ask: (run: () => unknown) => Promise.resolve(run()).then((data) => ({ data, error: null })),
  commands: {
    folderGlance: () => ({ branch: 'main', files: 2, added: 5, removed: 1 }),
    folderComplete: (...args: unknown[]) => complete(...args),
  },
}))

import { CommandInput } from './CommandInput'

afterEach(() => {
  cleanup()
  complete.mockReset()
})

function editor(onSubmit = vi.fn()) {
  render(<CommandInput cwd="/home/me/work" home="/home/me" history={['git status', 'ls -la']} settled={1} onSubmit={onSubmit} onClear={vi.fn()} onClassic={vi.fn()} />)
  return { field: screen.getByLabelText('Command') as HTMLTextAreaElement, onSubmit }
}

describe('the terminal’s editor', () => {
  it('shows where it is: the folder as ~, the branch and how far from HEAD', async () => {
    editor()
    expect(screen.getByText('~/work')).toBeTruthy()
    expect(await screen.findByText('main')).toBeTruthy()
    expect(screen.getByText('+5')).toBeTruthy()
  })

  it('sends on Enter, and walks history with ↑', () => {
    const { field, onSubmit } = editor()
    fireEvent.keyDown(field, { key: 'ArrowUp' })
    expect(field.value).toBe('git status')
    fireEvent.keyDown(field, { key: 'Enter' })
    expect(onSubmit).toHaveBeenCalledWith('git status')
    expect(field.value).toBe('')
  })

  it('offers the rest of a line from history, and Tab takes it', () => {
    const { field } = editor()
    fireEvent.change(field, { target: { value: 'git st' } })
    expect(screen.getByText('atus')).toBeTruthy()
    fireEvent.keyDown(field, { key: 'Tab' })
    expect(field.value).toBe('git status')
  })

  it('finishes a path with Tab, and lists the choices when there are several', async () => {
    const { field } = editor()
    complete.mockReturnValue(['src/main.rs', 'src/mod.rs'])
    fireEvent.change(field, { target: { value: 'cat src/' } })
    field.setSelectionRange(8, 8)
    await act(async () => {
      fireEvent.keyDown(field, { key: 'Tab' })
    })
    expect(complete).toHaveBeenCalledWith('/home/me/work', 'src/')
    expect(field.value).toBe('cat src/m')
    expect(screen.getByRole('listbox', { name: 'Completions' })).toBeTruthy()
    fireEvent.keyDown(field, { key: 'Enter' })
    expect(field.value).toBe('cat src/main.rs')
  })
})
