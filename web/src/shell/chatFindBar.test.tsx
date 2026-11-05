import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { useRef } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { ChatFind } from './ChatFind'

const found = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: { chatFind: (...args: unknown[]) => found(...args) },
}))

function Bar({ jump }: { jump: (id: string) => Promise<boolean> }): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  return (
    <div ref={box}>
      <ChatFind where={{ projectId: 'p', conversationId: 'c' }} box={box} jump={jump} onDone={() => {}} />
    </div>
  )
}

beforeEach(() => vi.useFakeTimers())
afterEach(() => {
  cleanup()
  vi.useRealTimers()
  found.mockReset()
})

describe('the find bar of a conversation', () => {
  it('counts the matches the whole conversation holds and walks them round', async () => {
    found.mockReturnValue({ hits: [{ messageId: 'm12', count: 2 }, { messageId: 'm1990', count: 1 }] })
    const jump = vi.fn(async () => true)
    render(<Bar jump={jump} />)
    fireEvent.change(screen.getByRole('textbox', { name: 'Find in the conversation' }), { target: { value: 'parser' } })
    await act(async () => {
      await vi.advanceTimersByTimeAsync(250)
    })
    expect(found).toHaveBeenCalledWith('p', 'c', 'parser')
    expect(screen.getByText('3 of 3')).toBeTruthy()
    expect(jump).toHaveBeenLastCalledWith('m1990', expect.any(Function))

    fireEvent.click(screen.getByRole('button', { name: 'Next match' }))
    expect(screen.getByText('1 of 3')).toBeTruthy()
    expect(jump).toHaveBeenLastCalledWith('m12', expect.any(Function))
  })
})
