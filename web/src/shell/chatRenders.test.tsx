import { act, cleanup, render } from '@testing-library/react'
import { useRef } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Frame, Message } from '../gen/bindings'
import { chatMessages, type ChatMessages } from './chatMessages'
import { ChatThread } from './ChatThread'
import { appliedAll } from './frames'

/* Every answer drawn draws its apps, so an answer drawn is a `TurnApps` drawn. */
const drawn = new Map<string, number>()
vi.mock('./TurnApps', () => ({
  TurnApps: ({ messageId }: { messageId: string }) => (drawn.set(messageId, (drawn.get(messageId) ?? 0) + 1), null),
}))
vi.mock('./TurnDelegations', () => ({ TurnDelegations: () => null }))
vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))
/* Every block drawn reads its text into spans. */
const read: string[] = []
vi.mock('./markdown', async (real) => {
  const markdown = await real<typeof import('./markdown')>()
  return { ...markdown, spans: (text: string) => (read.push(text), markdown.spans(text)) }
})

const message = (at: number): Message =>
  ({
    id: `m${at}`,
    turnId: at % 2 ? `t${at}` : null,
    role: at % 2 ? 'assistant' : 'user',
    parts: [{ kind: 'text', text: `Answer ${at}.\n\n- a point\n- another`, parent: null }],
    createdAt: at,
    streaming: false,
  }) as Message

const live = {
  id: 'live',
  turnId: 'tl',
  role: 'assistant',
  parts: [{ kind: 'text', text: 'A closed first block.\n\nThe second', parent: null }],
  createdAt: 999,
  streaming: true,
} as Message

const nothing = (): void => {}

function Thread({ messages }: { messages: ChatMessages }): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  const top = useRef<HTMLDivElement>(null)
  return (
    <div ref={box}>
      <ChatThread messages={messages} box={box} top={top} older={false} stalled={false} onMore={nothing} sending error={null} folder={null} rewindable={[]} onRewind={nothing} onEdit={nothing} onOpen={nothing} onRetry={nothing} onContinue={nothing} />
    </div>
  )
}

beforeEach(() => {
  vi.stubGlobal('matchMedia', () => ({ matches: true }))
})
afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
  drawn.clear()
  read.length = 0
})

describe('an answer streaming into a long thread', () => {
  it('draws no finished turn, and no closed block of its own, again for 500 frames of text', () => {
    const messages = chatMessages([...Array.from({ length: 199 }, (_, at) => message(at)), live])
    render(<Thread messages={messages} />)
    expect(drawn.get('m197')).toBe(1)
    drawn.clear()
    read.length = 0

    for (let at = 0; at < 500; at += 1) {
      const frame = { type: 'part', message_id: 'live', part: { kind: 'text', text: ' word', parent: null } } as Frame
      act(() => messages.set((was) => appliedAll(was, [frame])))
    }

    expect(drawn.get('live')).toBeGreaterThan(400)
    expect([...drawn.keys()].filter((id) => id !== 'live')).toEqual([])
    expect(read.filter((text) => text === 'A closed first block.')).toEqual([])
    expect(read.at(-1)).toMatch(/^The second( word){500}$/)
  }, 30_000)
})
