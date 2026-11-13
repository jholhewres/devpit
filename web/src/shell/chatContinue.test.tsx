import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { useRef } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { CONTINUE, continuable } from './chat'
import { chatMessages } from './chatMessages'
import { ChatThread } from './ChatThread'
import { rangesIn } from './chatSearch'
import { Markdown } from './MarkdownView'
import { Turn } from './Turn'
import { discloseMessage } from './useDisclosed'

vi.mock('./TurnApps', () => ({ TurnApps: () => null }))
vi.mock('./TurnDelegations', () => ({ TurnDelegations: () => null }))
vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))

const asked = { id: 'm1', turnId: 't1', role: 'user', parts: [{ kind: 'text', text: 'Fix the build', parent: null }], createdAt: 1, streaming: false } as Message
const stopped = { id: 'm2', turnId: 't1', role: 'assistant', parts: [{ kind: 'text', text: 'Half', parent: null }], createdAt: 2, streaming: false, ended: { at: 2, stopReason: 'interrupted', isError: false } } as Message
const done = { ...stopped, ended: { at: 2, stopReason: 'end_turn', isError: false } } as Message

function Thread({ messages, onContinue }: { messages: readonly Message[]; onContinue: () => void }): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  const top = useRef<HTMLDivElement>(null)
  const nothing = (): void => {}
  return (
    <div ref={box}>
      <ChatThread messages={chatMessages(messages)} box={box} top={top} older={false} stalled={false} onMore={nothing} sending={false} error={null} folder={null} rewindable={[]} onRewind={nothing} onEdit={nothing} onOpen={nothing} onRetry={nothing} onContinue={onContinue} />
    </div>
  )
}

beforeEach(() => vi.stubGlobal('matchMedia', () => ({ matches: true })))
afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
})

describe('a conversation that stopped part way', () => {
  it('offers Continue under "No answer was saved", which starts the next turn', () => {
    const go = vi.fn()
    render(<Thread messages={[asked]} onContinue={go} />)
    expect(screen.getByText(/No answer was saved/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }))
    expect(go).toHaveBeenCalledOnce()
  })

  it('is picked up after a stopped answer, never after a finished one or while a turn runs', () => {
    expect(continuable([asked, stopped], false)).toBe(true)
    expect(continuable([asked, done], false)).toBe(false)
    expect(continuable([asked], true)).toBe(false)
    expect(CONTINUE).toMatch(/last step/)
  })
})

describe('a code block', () => {
  it('copies what is between the fences, never the fences', async () => {
    const writeText = vi.fn(async () => {})
    vi.stubGlobal('navigator', { clipboard: { writeText } })
    render(<Markdown source={'Run this:\n\n```sh\nmake test\nmake fmt\n```\n'} />)
    fireEvent.click(screen.getByRole('button', { name: 'Copy code' }))
    expect(writeText).toHaveBeenCalledWith('make test\nmake fmt')
    expect(await screen.findByRole('button', { name: 'Copied' })).toBeTruthy()
  })
})

describe('a match in a settled turn', () => {
  it('is drawn once the search opens the fold that hid it', () => {
    const narrated = {
      id: 'm9',
      turnId: 't9',
      role: 'assistant',
      createdAt: 2,
      streaming: false,
      ended: { at: 2, stopReason: 'end_turn', isError: false },
      parts: [
        { kind: 'text', text: 'I will check the parser first.', parent: null },
        { kind: 'tool_call', id: 'c1', name: 'Read', input: '{}', state: 'ok', parent: null },
        { kind: 'tool_result', call_id: 'c1', output: 'x', is_error: false, parent: null },
        { kind: 'text', text: 'Done.', parent: null },
      ],
    } as unknown as Message
    render(<Turn message={narrated} folder={null} />)
    expect(rangesIn(document.body, 'parser')).toHaveLength(0)
    act(() => discloseMessage('m9'))
    expect(rangesIn(document.body, 'parser')).toHaveLength(1)
  })
})

describe('a match in a long prompt', () => {
  it('is unfolded from under Show more when the search lands in it', () => {
    const asked = {
      id: 'u1',
      turnId: 't1',
      role: 'user',
      createdAt: 1,
      streaming: false,
      ended: null,
      parts: [{ kind: 'text', text: Array.from({ length: 20 }, (_, at) => (at === 19 ? 'the parser' : `line ${at}`)).join('\n'), parent: null }],
    } as unknown as Message
    render(<Turn message={asked} folder={null} />)
    expect(document.querySelector('.said__t')!.hasAttribute('data-folded')).toBe(true)
    act(() => discloseMessage('u1'))
    expect(document.querySelector('.said__t')!.hasAttribute('data-folded')).toBe(false)
  })
})
