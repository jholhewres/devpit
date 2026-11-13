import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { ChatQueue, Message } from '../gen/bindings'
import { ChatPane } from './ChatPane'

/* What a step started — the queue read from disk, a steer's answer, a write —
   is settled before the next step looks, instead of polled for under a time
   budget a loaded runner outruns. Every mock answers with a promise, so one
   turn of the event loop inside act drains them, and act draws what they set. */
const settled = (): Promise<void> => act(async () => void (await new Promise((done) => setTimeout(done, 0))))
/* Only a hang guard: nothing below waits on time. Rendering the pane is slow under load. */
const LIMIT = 30_000

const user = (id: string, text: string): Message =>
  ({ id, turnId: null, role: 'user', parts: [{ kind: 'text', text, parent: null }], createdAt: 0, streaming: false }) as Message
const answer = (id: string): Message =>
  ({ id, turnId: 't', role: 'assistant', parts: [{ kind: 'text', text: 'ok', parent: null }], createdAt: 0, streaming: false }) as Message

const picture = { name: 'pasted-1.png', path: '/h/pasted/pasted-1.png', kind: 'png' }
const chat = {
  messages: [user('u1', 'first prompt'), answer('a1'), user('u2', '@/w/a.ts\nsecond prompt'), answer('a2'), user('u3', 'last prompt'), answer('a3')],
  rewindable: [], files: [] as typeof picture[], skills: [], previews: {}, asked: [], error: null, sending: false, failed: false,
  profileId: 'prof_1', profiles: [], cost: 0, session: null, card: null, older: false, folder: null,
  say: vi.fn(), steer: vi.fn(async () => true), stop: vi.fn(), answer: vi.fn(), detach: vi.fn(), attach: vi.fn(), paste: vi.fn(),
  keep: vi.fn(), clearExtras: vi.fn(), setSkills: vi.fn(), rewind: vi.fn(), took: vi.fn(),
}
afterEach(() => {
  cleanup()
  localStorage.clear()
  chat.sending = false
  chat.files = []
  vi.clearAllMocks()
})

const disk: { queue: ChatQueue } = { queue: { items: [], paused: false } }
vi.mock('./live', async (actual) => ({
  ...(await actual<typeof import('./live')>()),
  ask: async (call: () => unknown) => {
    try {
      return { data: await call(), error: null, loading: false }
    } catch {
      return { data: null, error: 'not here', loading: false }
    }
  },
  commands: {
    chatQueue: async () => disk.queue,
    chatQueueKeep: async (_project: string, _id: string, queue: ChatQueue) => ((disk.queue = queue), queue),
  },
}))
vi.mock('./useChat', () => ({ useChat: () => chat }))
vi.mock('./useSlash', () => ({ useSlash: () => ({ keyDown: () => false, ran: () => false }) }))
vi.mock('./useStop', () => ({ useStop: () => false }))
vi.mock('./useShell', () => ({
  useShell: () => ({ active: { id: 'conv_1' }, project: { id: 'p1', name: 'devpit' }, rename: vi.fn(), drafted: vi.fn() }),
}))
vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))
vi.mock('./Asked', () => ({ Asked: () => null }))
vi.mock('./Chips', () => ({ Chips: () => null }))
vi.mock('./ChatThread', () => ({ ChatThread: () => null }))
vi.mock('./ChatWhere', () => ({ ChatWhere: () => null }))
vi.mock('./ComposerStatus', () => ({ ComposerStatus: () => null }))
vi.mock('./ComposerTools', () => ({ ComposerTools: () => null }))
vi.mock('./CopySession', () => ({ CopySession: () => null }))
vi.mock('./DropTarget', () => ({ DropTarget: () => null }))
vi.mock('./PaneCorner', () => ({ PaneCorner: () => null }))
vi.mock('./SkillPills', () => ({ SkillPills: () => null }))
vi.mock('./SlashMenu', () => ({ SlashMenu: () => null }))
vi.mock('./MentionMenu', () => ({ MentionMenu: () => null }))

const field = (): HTMLTextAreaElement => screen.getByPlaceholderText(/^Do anything…/) as HTMLTextAreaElement

describe('Mod+Shift+Enter with a turn running', () => {
  it('steers the first queued message in with its files, and a held-down key steers only once', async () => {
    chat.sending = true
    const first = { id: 'q1', prompt: 'look at this', files: [picture], skills: [] }
    disk.queue = { items: [first, { id: 'q2', prompt: 'then this', files: [], skills: [] }], paused: false }
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    await settled()
    expect(screen.getByText('look at this')).toBeTruthy()

    fireEvent.keyDown(field(), { key: 'Enter', ctrlKey: true, shiftKey: true })
    await settled()
    expect(screen.queryByText('look at this')).toBeNull()
    expect(chat.steer).toHaveBeenCalledTimes(1)
    expect(chat.steer).toHaveBeenCalledWith('look at this', first)
    expect(chat.say).not.toHaveBeenCalled()

    fireEvent.keyDown(field(), { key: 'Enter', ctrlKey: true, shiftKey: true, repeat: true })
    await settled()
    expect(chat.steer).toHaveBeenCalledTimes(1)
    expect(screen.getByText('then this')).toBeTruthy()
  }, LIMIT)

  it('Mod+Enter steers what is typed, with what is attached, instead of queueing it', async () => {
    chat.sending = true
    chat.files = [picture]
    disk.queue = { items: [], paused: false }
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    fireEvent.change(field(), { target: { value: 'now please' } })
    fireEvent.keyDown(field(), { key: 'Enter', ctrlKey: true })
    expect(chat.steer).toHaveBeenCalledWith('now please', { files: [picture], skills: [] })
    expect(chat.clearExtras).toHaveBeenCalled()
    expect(field().value).toBe('')
  })
})

describe('a stop with messages queued', () => {
  it('holds the queue, and giving it back puts the words and files in the composer', async () => {
    chat.sending = true
    disk.queue = { items: [{ id: 'q1', prompt: 'wait for me', files: [picture], skills: [] }], paused: false }
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    await settled()
    expect(screen.getByText('wait for me')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Stop' }))
    await settled()
    expect(screen.getByText('Queue paused')).toBeTruthy()
    expect(chat.stop).toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Back to composer' }))
    expect(field().value).toBe('wait for me')
    expect(chat.keep).toHaveBeenCalledWith(picture)
    await settled()
    expect(disk.queue.items).toEqual([])
  }, LIMIT)
})

describe('↑ and ↓ in an empty composer', () => {
  it('bring back the penultimate prompt, then the last, then the empty composer', () => {
    disk.queue = { items: [], paused: false }
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    fireEvent.keyDown(field(), { key: 'ArrowUp' })
    expect(field().value).toBe('last prompt')
    fireEvent.keyDown(field(), { key: 'ArrowUp' })
    expect(field().value).toBe('second prompt')
    fireEvent.keyDown(field(), { key: 'ArrowDown' })
    expect(field().value).toBe('last prompt')
    fireEvent.keyDown(field(), { key: 'ArrowDown' })
    expect(field().value).toBe('')
  })

  it('leave the caret alone once something is typed or attached', () => {
    disk.queue = { items: [], paused: false }
    const { unmount } = render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    fireEvent.change(field(), { target: { value: 'my own words' } })
    fireEvent.keyDown(field(), { key: 'ArrowUp' })
    expect(field().value).toBe('my own words')
    unmount()

    chat.files = [picture]
    render(<ChatPane tab={{ id: 'conv_2', kind: 'chat' }} />)
    fireEvent.keyDown(field(), { key: 'ArrowUp' })
    expect(field().value).toBe('')
  })
})

describe('a paste of a megabyte of text', () => {
  it('is kept as a file to attach, never put in the field, and returns inside one long task', async () => {
    disk.queue = { items: [], paused: false }
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    const megabyte = 'x'.repeat(1024 * 1024)
    const clipboardData = { items: [], getData: (type: string) => (type === 'text/plain' ? megabyte : '') }
    // The work on this thread, not the wall clock: a loaded machine
    // stretched the same work past 50 ms while it waited for a core.
    let took = 0
    act(() => {
      const start = process.threadCpuUsage()
      fireEvent.paste(field(), { clipboardData })
      const spent = process.threadCpuUsage(start)
      took = (spent.user + spent.system) / 1000
    })
    expect(took).toBeLessThan(50)
    expect(field().value).toBe('')
    expect(chat.paste).toHaveBeenCalledTimes(1)
    const kept = chat.paste.mock.calls[0]![0] as Blob
    expect(kept.type).toBe('text/plain')
    expect(kept.size).toBe(megabyte.length)
  })

  it('a short paste is still typing', () => {
    disk.queue = { items: [], paused: false }
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    const clipboardData = { items: [], getData: (type: string) => (type === 'text/plain' ? 'short' : '') }
    const event = fireEvent.paste(field(), { clipboardData })
    expect(event).toBe(true)
    expect(chat.paste).not.toHaveBeenCalled()
  })
})
