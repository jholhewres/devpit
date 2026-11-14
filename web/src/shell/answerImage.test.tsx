import { act, cleanup, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { Turn } from './Turn'

vi.mock('./TurnApps', () => ({ TurnApps: () => null }))
vi.mock('./TurnDelegations', () => ({ TurnDelegations: () => null }))
vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))
const streamed: string[] = []
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => call(),
  commands: {
    mediaOpen: async (_project: string | null, path: string) => {
      streamed.push(path)
      return { data: `tok${streamed.length}`, error: null }
    },
  },
}))
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (token: string, scheme: string) => `${scheme}://localhost/${token}` }))

const answer = (text: string): Message =>
  ({ id: 'm2', turnId: 't1', role: 'assistant', parts: [{ kind: 'text', text, parent: null }], createdAt: 1, streaming: false, ended: { at: 2, stopReason: 'end_turn', isError: false } }) as Message

beforeEach(() => {
  streamed.length = 0
  vi.stubGlobal('matchMedia', () => ({ matches: true }))
})
afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
})

describe('a picture an answer names on this machine', () => {
  it('draws a local PNG, read through the backend', async () => {
    const { container } = render(<Turn message={answer('The page now:\n\n![after](/w/card/shots/after.png)')} folder="/w/card" />)
    await act(async () => {})
    expect(streamed).toEqual(['/w/card/shots/after.png'])
    expect(container.querySelector<HTMLImageElement>('img.md__img')?.src).toBe('devpitmedia://localhost/tok1')
  })

  it('reads one named from the folder the conversation runs in', async () => {
    render(<Turn message={answer('![before](shots/before.png)')} folder="/w/card" />)
    await act(async () => {})
    expect(streamed).toEqual(['/w/card/shots/before.png'])
  })

  it('leaves a path that is not a picture as its words', async () => {
    const { container } = render(<Turn message={answer('![notes](/w/card/notes.txt)')} folder="/w/card" />)
    await act(async () => {})
    expect(streamed).toEqual([])
    expect(container.querySelector('img')).toBeNull()
    expect(container.textContent).toContain('/w/card/notes.txt')
  })
})
