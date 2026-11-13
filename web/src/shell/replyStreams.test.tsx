import { cleanup, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { Turn } from './Turn'

vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))
vi.mock('./TurnApps', () => ({ TurnApps: () => null }))
vi.mock('./TurnDelegations', () => ({ TurnDelegations: () => null }))

const answer = (text: string, streaming: boolean): Message =>
  ({ id: 'a1', turnId: 't1', role: 'assistant', parts: [{ kind: 'text', text, parent: null }], createdAt: 0, streaming }) as Message

const motion = (reduce: boolean): void => {
  vi.stubGlobal('matchMedia', (query: string) => ({ matches: reduce && query.includes('reduce') }))
}

beforeEach(() => vi.useFakeTimers())
afterEach(() => {
  cleanup()
  vi.useRealTimers()
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe('an answer that is still arriving', () => {
  it('shows every piece the moment it arrives, with no tick to wait for', () => {
    motion(false)
    const ticks = vi.spyOn(window, 'setInterval')
    const view = render(<Turn message={answer('The first block.', true)} />)
    view.rerender(<Turn message={answer('The first block.\n\nThen a second one arrives.', true)} />)

    expect(view.container.querySelector('.reply')?.textContent).toContain('Then a second one arrives.')
    expect(ticks).not.toHaveBeenCalled()
    expect(view.container.querySelector('.reply__caret')).toBeNull()
  })

  it('fades its new blocks in', () => {
    motion(false)
    const view = render(<Turn message={answer('A block.', true)} />)
    expect(view.container.querySelector('.reply')?.hasAttribute('data-fade')).toBe(true)
  })

  it('shows them at once, with no fade, when motion is reduced', () => {
    motion(true)
    const view = render(<Turn message={answer('A block.', true)} />)
    view.rerender(<Turn message={answer('A block.\n\nAnd the next.', true)} />)

    expect(view.container.querySelector('.reply')?.textContent).toContain('And the next.')
    expect(view.container.querySelector('.reply')?.hasAttribute('data-fade')).toBe(false)
  })
})

describe('a settled answer', () => {
  it('does not fade when it is drawn', () => {
    motion(false)
    const view = render(<Turn message={answer('Done.', false)} />)
    expect(view.container.querySelector('.reply')?.hasAttribute('data-fade')).toBe(false)
  })
})
