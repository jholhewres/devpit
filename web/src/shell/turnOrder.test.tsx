import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Ended, Message, Part } from '../gen/bindings'
import { appliedAll, withFiles } from './chat'
import { Turn } from './Turn'

vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))
const pictures: string[] = []
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    mediaOpen: (_project: string | null, path: string) => {
      pictures.push(path)
      return 'tok1'
    },
  },
}))
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (token: string, scheme: string) => `${scheme}://localhost/${token}` }))

afterEach(() => {
  cleanup()
  vi.useRealTimers()
})

const text = (said: string): Part => ({ kind: 'text', text: said, parent: null })
const call = (id: string, name: string, input: object, ok = true): Part[] => [
  { kind: 'tool_call', id, name, input: JSON.stringify(input), state: ok ? 'ok' : 'failed', parent: null },
  { kind: 'tool_result', call_id: id, output: ok ? 'fine' : 'exit 1: no such file', is_error: !ok, parent: null },
]
const ORDERED: Part[] = [
  text('First I look.'),
  ...call('b1', 'Bash', { command: 'ls src' }),
  text('Then I change it.'),
  ...call('e1', 'Edit', { file_path: 'src/main.rs', old_string: 'a', new_string: 'b' }),
  text('Done: it builds.'),
]
const BEGAN = 1_793_800_000_000
const answer = (parts: Part[], over: Partial<Message> = {}): Message => ({
  id: 'm1',
  turnId: 't1',
  role: 'assistant',
  parts,
  createdAt: BEGAN,
  streaming: false,
  ...over,
})
const ended = (seconds: number, over: Partial<Ended> = {}): Ended => ({
  at: BEGAN / 1000 + seconds,
  stopReason: 'end_turn',
  isError: false,
  error: null,
  ...over,
})

/* What the turn draws, top to bottom: prose by its words, work by its header. */
const order = (container: HTMLElement): string[] =>
  [...container.querySelectorAll('.turn > .reply, .turn > .acts')].map((one) =>
    one.classList.contains('reply') ? `text:${one.textContent}` : 'work',
  )

describe('a turn in the order it happened', () => {
  it('draws text, Bash, text, Edit, text in that order while it runs', () => {
    const { container } = render(<Turn message={answer(ORDERED, { streaming: true })} />)
    expect(order(container)).toEqual(['text:First I look.', 'work', 'text:Then I change it.', 'work', 'text:Done: it builds.'])
  })

  it('once over, shows the last prose and a line that opens the rest, in order', () => {
    const { container } = render(<Turn message={answer(ORDERED, { ended: ended(90) })} />)
    expect(order(container)).toEqual(['text:Done: it builds.'])
    const line = screen.getByRole('button', { name: /Worked for 1m 30s/ })
    expect(line.getAttribute('aria-expanded')).toBe('false')
    expect(line.textContent).toContain('1 command · 1 file edit')

    fireEvent.click(line)
    expect(order(container)).toEqual(['text:First I look.', 'work', 'text:Then I change it.', 'work', 'text:Done: it builds.'])
  })
})

describe('how long it worked', () => {
  it('counts from when the turn began, so a reload 90 s in shows about 90 s', () => {
    vi.useFakeTimers({ now: BEGAN + 90_000 })
    render(<Turn message={answer(ORDERED.slice(0, 3), { streaming: true })} />)
    expect(screen.getByText('· 1m 30s')).toBeTruthy()
    act(() => vi.advanceTimersByTime(2_100))
    expect(screen.getByText('· 1m 32s')).toBeTruthy()
  })

  it('reads a turn kept in seconds as one kept in milliseconds', () => {
    vi.useFakeTimers({ now: BEGAN + 45_000 })
    render(<Turn message={answer(ORDERED.slice(0, 1), { streaming: true, createdAt: BEGAN / 1000 })} />)
    expect(screen.getByText('· 45s')).toBeTruthy()
  })

  it('says "Worked for 1m 30s" when the end frame comes, and keeps saying it', () => {
    vi.useFakeTimers({ now: BEGAN + 90_000 })
    const live = [answer(ORDERED, { streaming: true })]
    const [over] = appliedAll(live, [{ type: 'ended', end: { turnId: 't1', costUsd: null, durationMs: null, stopReason: 'end_turn', isError: false, context: null } }])
    expect(over!.streaming).toBe(false)
    render(<Turn message={over!} />)
    expect(screen.getByRole('button', { name: /Worked for 1m 30s/ })).toBeTruthy()
  })

  it('says it was interrupted, and when', () => {
    render(<Turn message={answer(ORDERED.slice(0, 3), { ended: ended(12, { stopReason: 'cancelled' }) })} />)
    expect(screen.getByRole('button', { name: /Interrupted after 12s/ })).toBeTruthy()
  })

  it('says nothing of a duration an old transcript never kept', () => {
    render(<Turn message={answer(ORDERED)} />)
    const line = screen.getByRole('button', { name: /Worked/ })
    expect(line.textContent).not.toMatch(/\d+s/)
  })
})

describe('a turn that failed', () => {
  it('ends on its error, open, with nothing to expand', () => {
    const parts = [text('Running the tests.'), ...call('b1', 'Bash', { command: 'cargo test' }, false)]
    const { container } = render(<Turn message={answer(parts, { ended: ended(7, { isError: true, stopReason: 'error', error: 'claude exited with status 1' }) })} />)
    const alert = screen.getByRole('alert')
    expect(alert.textContent).toContain('Failed after 7s')
    expect(alert.textContent).toContain('claude exited with status 1')
    // Nothing folded: the prose and the work it failed on are both on screen.
    expect(order(container)).toEqual(['text:Running the tests.', 'work'])
    expect(container.querySelector('.acts__h')!.getAttribute('aria-expanded')).toBe('true')
    expect(screen.getByText('cargo test')).toBeTruthy()
    expect(screen.queryByRole('button', { name: /Failed/ })).toBeNull()
  })
})

describe('what a person attached', () => {
  const pasted = '/home/me/.devpit/projects/p1/pasted/pasted-01m6.png'
  const said = (prompt: string): Message => ({
    id: 'u1',
    turnId: 't1',
    role: 'user',
    parts: [text(prompt)],
    createdAt: BEGAN,
    streaming: false,
  })

  it('shows a pasted picture as a thumbnail, and a file as a chip, not as @paths', async () => {
    // The text a sent turn and a reopened conversation both hold.
    const sent = withFiles('what is wrong here?', [
      { name: 'pasted-01m6.png', path: pasted, kind: 'png' },
      { name: 'main.rs', path: 'src/main.rs', kind: 'rs' },
    ])
    const { container } = render(<Turn message={said(sent)} />)
    await act(async () => {})
    const img = container.querySelector<HTMLImageElement>('.said__pic img')!
    expect(img.src).toBe('devpitmedia://localhost/tok1')
    expect(pictures).toContain(pasted)
    expect(screen.getByText('main.rs')).toBeTruthy()
    expect(container.querySelector('.said__t')!.textContent).toBe('what is wrong here?')
    expect(container.textContent).not.toContain('@')
  })

  it('folds a long message, with a button to read the rest', () => {
    const long = Array.from({ length: 40 }, (_, at) => `line ${at}`).join('\n')
    const { container } = render(<Turn message={said(long)} />)
    expect(container.querySelector('.said__t')!.hasAttribute('data-folded')).toBe(true)
    fireEvent.click(screen.getByRole('button', { name: 'Show more' }))
    expect(container.querySelector('.said__t')!.hasAttribute('data-folded')).toBe(false)
  })
})

describe('a turn that fails while it is watched', () => {
  it('keeps the error on its answer, as a reopened one would', async () => {
    const { failedOn } = await import('./chat')
    const [failed] = failedOn([answer([text('Half')], { streaming: true })], 'm1', 'claude exited with status 1')
    render(<Turn message={failed!} />)
    expect(screen.getByRole('alert').textContent).toContain('claude exited with status 1')
  })
})
