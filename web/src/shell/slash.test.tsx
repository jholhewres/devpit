import { act, cleanup, fireEvent, render, renderHook, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { matching, picked, slashQuery } from './slash'
import { SlashMenu } from './SlashMenu'
import { useSlash } from './useSlash'

afterEach(cleanup)

/* A catalogue like the recorded init line's: `compact`, `clear`, `review`, `tdd`. */
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: { chatSlashCommands: () => ['compact', 'clear', 'review', 'tdd'] },
}))

describe('what the composer asks for', () => {
  it('is the word after a leading slash, until a space', () => {
    expect(slashQuery('/')).toBe('')
    expect(slashQuery('/co')).toBe('co')
    // With a space it is the command's arguments, and the menu is done.
    expect(slashQuery('/compact now')).toBeNull()
    expect(slashQuery('fix the /co')).toBeNull()
  })

  it('offers commands starting with it before ones merely containing it', () => {
    expect(matching(['review', 'clear', 'compact', 'recompile'], 'c')).toEqual(['clear', 'compact', 'recompile'])
  })

  it('leaves a space after a picked command for its arguments', () => {
    expect(picked('compact')).toBe('/compact ')
  })
})

describe('the slash menu', () => {
  it('lists the profile’s commands and picks one with Enter', async () => {
    let prompt = '/c'
    const setPrompt = vi.fn((next: string) => (prompt = next))
    const { result } = renderHook(() => useSlash('claude', prompt, setPrompt))
    await waitFor(() => expect(result.current.items).toEqual(['compact', 'clear']))

    const enter = { key: 'Enter', preventDefault: vi.fn(), nativeEvent: {} } as unknown as React.KeyboardEvent
    act(() => {
      expect(result.current.keyDown(enter)).toBe(true)
    })
    expect(setPrompt).toHaveBeenCalledWith('/compact ')
  })

  /* The CLI calls /resume terminal-only, so the chat never offered it; it is
     devpit's own now, and opens the picker instead of reaching the CLI. */
  it('offers /resume, and picking it opens the picker instead of typing it', async () => {
    const setPrompt = vi.fn()
    const { result } = renderHook(() => useSlash('claude', '/res', setPrompt))
    await waitFor(() => expect(result.current.items).toEqual(['resume']))
    act(() => result.current.choose('resume'))
    expect(setPrompt).toHaveBeenCalledWith('')
    expect(result.current.own).toBe('resume')
    act(() => result.current.closeOwn())
    expect(result.current.own).toBeNull()
  })

  it('does not take Enter when nothing matches, so the message still sends', async () => {
    const { result } = renderHook(() => useSlash('claude', 'hello', vi.fn()))
    await waitFor(() => expect(result.current.items).toEqual([]))
    const enter = { key: 'Enter', preventDefault: vi.fn(), nativeEvent: {} } as unknown as React.KeyboardEvent
    expect(result.current.keyDown(enter)).toBe(false)
  })

  it('draws the items and picks on press', () => {
    const choose = vi.fn()
    render(<SlashMenu slash={{ items: ['compact', 'clear'], at: 1, choose, keyDown: () => false, own: null, closeOwn: vi.fn() }} />)
    expect(screen.getByRole('option', { name: '/clear' }).getAttribute('aria-selected')).toBe('true')
    fireEvent.mouseDown(screen.getByRole('option', { name: '/compact' }))
    expect(choose).toHaveBeenCalledWith('compact')
  })
})
