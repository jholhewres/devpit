import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { useRef } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { CONTINUE, continuable } from './chat'
import { ChatThread } from './ChatThread'
import { Markdown } from './MarkdownView'

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
      <ChatThread messages={messages} box={box} top={top} older={false} stalled={false} onMore={nothing} sending={false} error={null} folder={null} rewindable={[]} onRewind={nothing} onOpen={nothing} onContinue={onContinue} />
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
