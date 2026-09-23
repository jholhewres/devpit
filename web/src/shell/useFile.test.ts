import { act, renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import { useFile } from './useFile'

/* A save the test answers when it chooses, to type while it is on its way. */
let answer: (value: { readAt: number; bytes: number; path: string }) => void = () => {}
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, code: null, loading: false }),
  commands: {
    fileRead: async () => ({ path: 'a.txt', fullPath: '/w/a.txt', text: 'one', kind: 'text', bytes: 3, readAt: 1, notShown: null, dataUrl: null }),
    fileWrite: () => new Promise((done) => (answer = done)),
  },
}))
const shell = { project: { id: 'p1' } }
vi.mock('./useShell', () => ({ useShell: () => shell }))

describe('a file being edited', () => {
  it('stays unsaved when it was typed into while a save was on its way', async () => {
    const { result } = renderHook(() => useFile('a.txt'))
    await waitFor(() => expect(result.current.text).toBe('one'))

    act(() => result.current.change('two'))
    expect(result.current.dirty).toBe(true)
    act(() => result.current.save())
    act(() => result.current.change('three'))
    await act(async () => answer({ readAt: 2, bytes: 3, path: 'a.txt' }))

    // "two" was saved; "three" was not, and closing the tab must still ask.
    expect(result.current.dirty).toBe(true)
    act(() => result.current.change('two'))
    expect(result.current.dirty).toBe(false)
  })
})
