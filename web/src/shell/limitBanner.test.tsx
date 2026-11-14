import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { useRef } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Limit, Message } from '../gen/bindings'
import { clock } from './chat'
import { chatMessages } from './chatMessages'
import { ChatThread } from './ChatThread'
import { endedNow } from './turnFold'

vi.mock('./TurnApps', () => ({ TurnApps: () => null }))
vi.mock('./TurnDelegations', () => ({ TurnDelegations: () => null }))
vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))

/* Unix seconds, as the CLI's `resetsAt` says it. */
const RESETS = 1_795_143_600
const asked = { id: 'm1', turnId: 't1', role: 'user', parts: [{ kind: 'text', text: 'Fix the build', parent: null }], createdAt: 1, streaming: false } as Message
const refused = (limit: Limit): Message =>
  ({
    id: 'm2',
    turnId: 't1',
    role: 'assistant',
    parts: [{ kind: 'text', text: "You've hit your session limit · resets 3:50pm", parent: null }],
    createdAt: 2,
    streaming: false,
    ended: { ...endedNow({ stopReason: 'success', isError: true, limit }), at: 3 },
  }) as Message

function Thread({ messages, onRetry }: { messages: readonly Message[]; onRetry: (text: string) => void }): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  const top = useRef<HTMLDivElement>(null)
  const nothing = (): void => {}
  return (
    <div ref={box}>
      <ChatThread messages={chatMessages(messages)} box={box} top={top} older={false} stalled={false} onMore={nothing} sending={false} error={null} folder={null} rewindable={[]} onRewind={nothing} onEdit={nothing} onOpen={nothing} onRetry={onRetry} onContinue={nothing} />
    </div>
  )
}

beforeEach(() => vi.stubGlobal('matchMedia', () => ({ matches: true })))
afterEach(() => {
  cleanup()
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('a turn the usage limit refused', () => {
  it('shows a banner with its reset time, and Try again sends the message again', () => {
    const retried = vi.fn()
    render(<Thread messages={[asked, refused({ kind: 'rate_limit', resetsAt: RESETS })]} onRetry={retried} />)
    const banner = screen.getByRole('region', { name: 'Usage limit reached' })
    expect(banner.textContent).toContain(`Resets at ${clock(RESETS * 1000)}`)
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }))
    expect(retried).toHaveBeenCalledWith('Fix the build')
  })

  it('resumes by itself once the limit resets, when asked to', () => {
    vi.useFakeTimers({ now: (RESETS - 60) * 1000 })
    const retried = vi.fn()
    render(<Thread messages={[asked, refused({ kind: 'rate_limit', resetsAt: RESETS })]} onRetry={retried} />)
    fireEvent.click(screen.getByRole('button', { name: 'Resume when it resets' }))
    act(() => void vi.advanceTimersByTime(59_000))
    expect(retried).not.toHaveBeenCalled()
    act(() => void vi.advanceTimersByTime(1_000))
    expect(retried).toHaveBeenCalledExactlyOnceWith('Fix the build')
  })
})

describe('a turn the provider failed', () => {
  it('offers Try again alone: an outage gives no time to wait for', () => {
    render(<Thread messages={[asked, refused({ kind: 'outage', resetsAt: null })]} onRetry={() => {}} />)
    expect(screen.getByRole('region', { name: 'The provider is not answering' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Try again' })).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Resume when it resets' })).toBeNull()
  })
})
