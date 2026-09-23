import { act, renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import { useFile } from './useFile'

/* Reads the test answers one by one, and a save that is refused. */
const reads: Array<{ path: string; done: (text: string) => void }> = []
let refusal: { message: string; code: string } | null = null
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => {
    try {
      return { data: await call(), error: null, code: null, loading: false }
    } catch (thrown) {
      const why = thrown as { message: string; code: string }
      return { data: null, error: why.message, code: why.code, loading: false }
    }
  },
  commands: {
    fileRead: (_p: string, _w: null, path: string) =>
      new Promise((done) =>
        reads.push({ path, done: (text) => done({ path, fullPath: `/w/${path}`, text, kind: 'text', bytes: 1, readAt: 1, notShown: null, dataUrl: null }) }),
      ),
    fileWrite: async () => {
      if (refusal) throw refusal
      return { readAt: 2, bytes: 1, path: 'b.txt' }
    },
  },
}))
const shell = { project: { id: 'p1' } }
vi.mock('./useShell', () => ({ useShell: () => shell }))

describe('a file tab', () => {
  it('shows the file it is on now, whatever order the reads answer in', async () => {
    const { result, rerender } = renderHook(({ path }) => useFile(path), { initialProps: { path: 'a.txt' } })
    rerender({ path: 'b.txt' })
    await waitFor(() => expect(reads).toHaveLength(2))
    await act(async () => reads[1]!.done('bee'))
    await act(async () => reads[0]!.done('ay'))
    expect(result.current.text).toBe('bee')
  })

  it('says a refused save as itself, not as a conflict to overwrite', async () => {
    reads.length = 0
    const { result } = renderHook(() => useFile('b.txt'))
    await waitFor(() => expect(reads).toHaveLength(1))
    await act(async () => reads[0]!.done('bee'))
    refusal = { message: 'permission denied', code: 'internal' }
    act(() => result.current.change('bees'))
    await act(async () => result.current.save())
    await waitFor(() => expect(result.current.error).toBe('permission denied'))
    expect(result.current.clash).toBeNull()
    refusal = null
  })
})
