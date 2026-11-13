import { act, fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import type { Part } from '../gen/bindings'
import { Acts } from './ActsRows'
import { WholeContext } from './cutOutput'

const asked: unknown[][] = []
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    chatPartFull: (...args: unknown[]) => {
      asked.push(args)
      return {
        kind: 'tool_result',
        call_id: 'call_1',
        output: JSON.stringify([{ type: 'image', source: { type: 'base64', media_type: 'image/png', data: 'iVBORw0KGgo=' } }]),
        is_error: false,
        parent: null,
      }
    },
  },
}))

const cut: Part[] = [
  { kind: 'tool_call', id: 'call_1', name: 'mcp__playwright__browser_take_screenshot', input: '{}', state: 'ok', parent: null },
  {
    kind: 'tool_result',
    call_id: 'call_1',
    output: JSON.stringify([{ type: 'image', mime: 'image/png', bytes: 300_000, cut: true }]),
    is_error: false,
    parent: null,
  },
]

describe('an output the window got cut', () => {
  it('is fetched whole when opened, and its image drawn only then', async () => {
    render(
      <WholeContext.Provider value={{ projectId: 'p1', conversationId: 'c1' }}>
        <Acts parts={cut} live={true} messageId="m1" />
      </WholeContext.Provider>,
    )
    fireEvent.click(screen.getByRole('button', { expanded: false }))
    expect(screen.queryByRole('img')).toBeNull()

    await act(async () => fireEvent.click(screen.getByRole('button', { name: /Show the image \(293 KB\)/ })))

    expect(asked).toEqual([['p1', 'c1', 'm1', 'call_1']])
    expect(screen.getByRole('img').getAttribute('src')).toBe('data:image/png;base64,iVBORw0KGgo=')
    fireEvent.click(screen.getByRole('button', { name: 'Enlarge Image 1 of the output' }))
    expect(screen.getByRole('dialog', { name: 'Image 1 of the output, enlarged' })).toBeTruthy()
  })

  it('offers nothing for an output that went whole', () => {
    const whole: Part[] = [cut[0]!, { ...cut[1]!, output: 'ok' } as Part]
    render(
      <WholeContext.Provider value={{ projectId: 'p1', conversationId: 'c1' }}>
        <Acts parts={whole} live={true} messageId="m2" />
      </WholeContext.Provider>,
    )
    fireEvent.click(screen.getByRole('button', { expanded: false }))
    expect(screen.queryByRole('button', { name: /Show/ })).toBeNull()
  })
})
