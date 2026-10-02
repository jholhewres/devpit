import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { ComposerTools } from './ComposerTools'
import type { Chat } from './useChat'

let heard: { text: string | null; note: string | null } = { text: 'fix the invoice total', note: null }
const pasted = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null }),
  commands: {
    chatPaste: (...args: unknown[]) => (pasted(...args), { name: 'voice-1.webm', path: '/h/pasted/voice-1.webm', kind: 'webm' }),
    chatTranscribe: () => heard,
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'p1' } }) }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(() => Promise.resolve(['/w/a.txt', '/w/b.png'])) }))
vi.mock('./recorder', async (actual) => ({
  ...(await actual<typeof import('./recorder')>()),
  formatHere: () => 'audio/webm;codecs=opus',
  record: () => Promise.resolve({ stop: () => Promise.resolve(new Blob(['x'], { type: 'audio/webm' })), cancel: vi.fn() }),
  base64Of: () => Promise.resolve('eA=='),
}))

afterEach(() => {
  cleanup()
  pasted.mockClear()
  heard = { text: 'fix the invoice total', note: null }
})

const chatWith = () => ({ attach: vi.fn(), keep: vi.fn(), detach: vi.fn() }) as unknown as Chat & { attach: ReturnType<typeof vi.fn>; keep: ReturnType<typeof vi.fn>; detach: ReturnType<typeof vi.fn> }

describe("the composer's files and voice", () => {
  it('attaches the files picked', async () => {
    const chat = chatWith()
    render(<ComposerTools chat={chat} onHeard={vi.fn()} />)
    fireEvent.click(screen.getByLabelText('Attach files'))
    await waitFor(() => expect(chat.attach).toHaveBeenCalledWith(['/w/a.txt', '/w/b.png']))
  })

  it('records, keeps the recording and puts what was heard in the composer', async () => {
    const chat = chatWith()
    const onHeard = vi.fn()
    render(<ComposerTools chat={chat} onHeard={onHeard} />)
    fireEvent.pointerDown(screen.getByLabelText('Record a voice message'))
    fireEvent.pointerUp(screen.getByLabelText('Record a voice message'))
    fireEvent.click(await screen.findByText('Done'))
    await waitFor(() => expect(onHeard).toHaveBeenCalledWith('fix the invoice total'))
    expect(pasted).toHaveBeenCalledWith('p1', 'audio/webm', 'eA==')
    expect(chat.keep).toHaveBeenCalled()
    expect(chat.detach).toHaveBeenCalledWith('/h/pasted/voice-1.webm')
  })

  it('sends the recording as a file, and says why, when it could not be heard', async () => {
    heard = { text: null, note: 'no whisper on this machine' }
    const chat = chatWith()
    render(<ComposerTools chat={chat} onHeard={vi.fn()} />)
    fireEvent.pointerDown(screen.getByLabelText('Record a voice message'))
    fireEvent.click(await screen.findByText('Done'))
    expect(await screen.findByText(/Sent as a recording: no whisper on this machine/)).toBeTruthy()
    expect(chat.keep).toHaveBeenCalled()
    expect(chat.detach).not.toHaveBeenCalled()
  })
})
