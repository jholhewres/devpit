import { act, cleanup, render, screen } from '@testing-library/react'
import { useRef } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { chatMessages, type ChatMessages } from './chatMessages'
import { ChatThread } from './ChatThread'

vi.mock('./TurnApps', () => ({ TurnApps: () => null }))
vi.mock('./TurnDelegations', () => ({ TurnDelegations: () => null }))
vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))

const asked = { id: 'm1', turnId: 't1', role: 'user', parts: [{ kind: 'text', text: 'Fix the build', parent: null }], createdAt: 1, streaming: false } as Message
const working = { id: 'm2', turnId: 't1', role: 'assistant', parts: [{ kind: 'text', text: 'Looking', parent: null }], createdAt: 2, streaming: true } as Message
const answered = { ...working, streaming: false, ended: { at: 14, stopReason: 'end_turn', isError: false } } as Message

function Thread({ messages }: { messages: ChatMessages }): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  const top = useRef<HTMLDivElement>(null)
  const nothing = (): void => {}
  return (
    <div ref={box}>
      <ChatThread messages={messages} box={box} top={top} older={false} stalled={false} onMore={nothing} sending={false} error={null} folder={null} rewindable={[]} onRewind={nothing} onEdit={nothing} onOpen={nothing} onRetry={nothing} onContinue={nothing} />
    </div>
  )
}

beforeEach(() => vi.stubGlobal('matchMedia', () => ({ matches: true })))
afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
})

describe('a conversation read with a screen reader', () => {
  it('announces a finished turn in a polite live region, not each streamed word', () => {
    const messages = chatMessages([asked, working])
    render(<Thread messages={messages} />)
    const live = screen.getByRole('status')
    expect(live.getAttribute('aria-live')).toBe('polite')
    expect(live.textContent).toBe('The agent is working.')

    act(() => messages.set([asked, { ...working, parts: [{ kind: 'text', text: 'Looking at it', parent: null }] } as Message]))
    expect(live.textContent).toBe('The agent is working.')

    act(() => messages.set([asked, answered]))
    expect(live.textContent).toBe('The agent answered. Worked for 12s')
  })

  it('is a log whose messages each say who spoke', () => {
    render(<Thread messages={chatMessages([asked, answered])} />)
    expect(screen.getByRole('log', { name: 'Conversation' })).toBeTruthy()
    expect(screen.getAllByRole('article', { name: 'You' })).toHaveLength(1)
    expect(screen.getAllByRole('article', { name: 'Agent' })).toHaveLength(1)
  })
})
