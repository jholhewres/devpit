import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from 'vitest'

import type { Conversation, LiveSession, Message, RemoteIn, RemoteOut, RemoteTalks } from '../gen/bindings'
import { RemoteChats } from './RemoteChats'
import { RemoteTalk } from './RemoteTalk'
import type { Talking } from './talk'

type Hears = { current: (heard: RemoteOut) => void }

const orchestrator: Talking = { kind: 'chat', project: 'orch', conversation: 'c1', title: 'work', orchestrator: 'claude-work' }
const session: Talking = { kind: 'session', profile: 'claude-work', name: 'api-worker', title: 'api-worker', project: 'api', pane: { projectId: 'api', paneId: '%4' } }

const transcript: Message[] = [
  { id: 'm1', turnId: 't1', role: 'user', parts: [{ kind: 'text', text: 'How is the api?', parent: null }], createdAt: 0, streaming: false },
  {
    id: 'm2',
    turnId: 't1',
    role: 'assistant',
    parts: [
      { kind: 'tool_call', id: 'k1', name: 'Bash', input: '{"command":"cargo test"}', state: 'ok', parent: null },
      { kind: 'tool_result', call_id: 'k1', output: 'ok', is_error: false, parent: null },
      { kind: 'text', text: 'All green on main.', parent: null },
    ],
    createdAt: 0,
    streaming: false,
  },
]

function shown(talking: Talking, options: { voice?: boolean; send?: Mock<(message: RemoteIn) => void> } = {}) {
  const send = options.send ?? vi.fn<(message: RemoteIn) => void>()
  const hears: Hears = { current: () => {} }
  render(<RemoteTalk talking={talking} typing voice={options.voice ?? false} drafts={[]} send={send} hears={hears} onBack={vi.fn()} onTerminal={vi.fn()} />)
  const say = (heard: RemoteOut): void => act(() => hears.current(heard))
  return { send, say }
}

const read = (messages: Message[]): RemoteOut => ({ t: 'chat', project: 'orch', conversation: { id: 'c1', messages } as unknown as Conversation })
const sentOf = (send: Mock<(message: RemoteIn) => void>, t: RemoteIn['t']): RemoteIn[] => send.mock.calls.map(([one]) => one).filter((one) => one.t === t)

afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
})

describe('the Remote chat', () => {
  it('reads the orchestrator’s transcript', () => {
    const { send, say } = shown(orchestrator)
    expect(send).toHaveBeenCalledWith({ t: 'chat', project: 'orch', conversation: 'c1' })
    say(read(transcript))
    expect(screen.getByText('How is the api?')).toBeTruthy()
    expect(screen.getByText('All green on main.')).toBeTruthy()
    expect(screen.getByText('Ran 1 command')).toBeTruthy()
  })

  it('shows the answer as it streams, and reads the turn again once it ends', () => {
    const { send, say } = shown(orchestrator)
    say(read(transcript))
    const frame = (frame: Extract<RemoteOut, { t: 'chatFrame' }>['frame']): void => say({ t: 'chatFrame', project: 'orch', conversation: 'c1', frame })
    frame({ type: 'opened', message: { id: 'm3', turnId: 't2', role: 'user', parts: [{ kind: 'text', text: 'And the site?', parent: null }], createdAt: 0, streaming: false } })
    frame({ type: 'opened', message: { id: 'm4', turnId: 't2', role: 'assistant', parts: [], createdAt: 0, streaming: true } })
    frame({ type: 'part', message_id: 'm4', part: { kind: 'text', text: 'The site ', parent: null } })
    frame({ type: 'part', message_id: 'm4', part: { kind: 'text', text: 'builds.', parent: null } })
    expect(screen.getByText('The site builds.')).toBeTruthy()
    expect(screen.getByText('Writing…')).toBeTruthy()
    // One turn at a time: the next waits for this one.
    expect((screen.getByText('Answering…') as HTMLButtonElement).disabled).toBe(true)
    send.mockClear()
    frame({ type: 'ended', end: { turnId: 't2', costUsd: null, durationMs: null, stopReason: null, isError: false, context: null } })
    expect(screen.queryByText('Writing…')).toBeNull()
    expect(send).toHaveBeenCalledWith({ t: 'chat', project: 'orch', conversation: 'c1' })
  })

  it('sends a typed message as the person’s turn, to the conversation or into the session', () => {
    const { send, say } = shown(orchestrator)
    say(read(transcript))
    fireEvent.change(screen.getByLabelText('Message'), { target: { value: 'Deploy it.' } })
    fireEvent.click(screen.getByText('Send'))
    expect(send).toHaveBeenCalledWith({ t: 'chatSay', project: 'orch', conversation: 'c1', text: 'Deploy it.' })
    expect((screen.getByLabelText('Message') as HTMLTextAreaElement).value).toBe('')
    cleanup()

    const other = shown(session)
    expect(other.send).toHaveBeenCalledWith({ t: 'session', profile: 'claude-work', name: 'api-worker' })
    fireEvent.change(screen.getByLabelText('Message'), { target: { value: 'yes' } })
    fireEvent.click(screen.getByText('Send'))
    expect(other.send).toHaveBeenCalledWith({ t: 'sessionSay', profile: 'claude-work', name: 'api-worker', text: 'yes' })
  })

  it('keeps a photo on the machine and names it in the message', async () => {
    const { send, say } = shown(orchestrator)
    const photo = new File([new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 1, 2, 3])], 'IMG_0001.jpg', { type: 'image/jpeg' })
    fireEvent.change(document.querySelector('input[type=file]')!, { target: { files: [photo] } })
    await waitFor(() => expect(sentOf(send, 'photo')).toHaveLength(1))
    const [kept] = sentOf(send, 'photo') as Extract<RemoteIn, { t: 'photo' }>[]
    expect(kept).toMatchObject({ project: 'orch', mime: 'image/jpeg' })
    const pieces = sentOf(send, 'uploadPart') as Extract<RemoteIn, { t: 'uploadPart' }>[]
    expect(pieces.every((one) => one.upload === kept!.upload)).toBe(true)
    expect(screen.getByText('Sending the photo…')).toBeTruthy()

    const path = '/home/me/.devpit/projects/orch/pasted/pasted-01k2.jpg'
    say({ t: 'attached', upload: kept!.upload, attachment: { name: 'pasted-01k2.jpg', path, kind: 'jpg' } })
    expect(screen.getByText('pasted-01k2.jpg ×')).toBeTruthy()
    fireEvent.change(screen.getByLabelText('Message'), { target: { value: 'What is this error?' } })
    fireEvent.click(screen.getByText('Send'))
    expect(send).toHaveBeenCalledWith({ t: 'chatSay', project: 'orch', conversation: 'c1', text: `@${path}\nWhat is this error?` })
  })

  it('refuses a photo too big to send before any of it goes', async () => {
    const { send } = shown(orchestrator)
    const big = new File([new Uint8Array(9 * 1024 * 1024)], 'scan.png', { type: 'image/png' })
    fireEvent.change(document.querySelector('input[type=file]')!, { target: { files: [big] } })
    await waitFor(() => expect(screen.getByText(/that photo is over 8 MB/)).toBeTruthy())
    expect(sentOf(send, 'uploadPart')).toHaveLength(0)
    expect(sentOf(send, 'photo')).toHaveLength(0)
  })
})

describe('who there is to talk to', () => {
  it('opens the orchestrator’s chat, and a session with its terminal a tap away', () => {
    const send = vi.fn<(message: RemoteIn) => void>()
    const onTerminal = vi.fn()
    const live = { name: 'api-worker', pid: 7, status: 'busy', projectId: 'api', projectName: 'api', pane: { projectId: 'api', paneId: '%4' }, waiting: null, draft: null } as unknown as LiveSession
    const talks: RemoteTalks = {
      orchestrators: [{ project: 'orch', name: 'work', profile: 'claude-work', conversation: 'c1' }],
      sessions: [{ profile: 'claude-work', session: live }],
      voice: false,
    }
    const hears: Hears = { current: () => {} }
    render(<RemoteChats project="api" chats={null} talks={talks} drafts={[]} typing send={send} hears={hears} onTerminal={onTerminal} />)
    expect(send).toHaveBeenCalledWith({ t: 'talks' })
    fireEvent.click(screen.getByText('work'))
    expect(send).toHaveBeenCalledWith({ t: 'chat', project: 'orch', conversation: 'c1' })
    fireEvent.click(screen.getByText('← Chats'))
    expect(screen.getByText('api · working')).toBeTruthy()
    fireEvent.click(screen.getByText('api-worker'))
    expect(send).toHaveBeenCalledWith({ t: 'session', profile: 'claude-work', name: 'api-worker' })
    fireEvent.click(screen.getByText('Terminal'))
    expect(onTerminal).toHaveBeenCalledWith({ projectId: 'api', paneId: '%4' })
  })
})

describe('dictation from the Remote', () => {
  let recording: Uint8Array

  beforeEach(() => {
    recording = new Uint8Array([1, 2, 3, 4])
    class Recorder {
      static isTypeSupported = (type: string): boolean => type === 'audio/webm;codecs=opus'
      state = 'inactive'
      mimeType: string
      ondataavailable: ((event: { data: Blob }) => void) | null = null
      onstop: (() => void) | null = null
      constructor(_stream: unknown, options?: { mimeType?: string }) {
        this.mimeType = options?.mimeType ?? ''
      }
      start(): void {
        this.state = 'recording'
      }
      stop(): void {
        this.state = 'inactive'
        this.ondataavailable?.({ data: new Blob([recording as BlobPart], { type: this.mimeType }) })
        this.onstop?.()
      }
    }
    vi.stubGlobal('MediaRecorder', Recorder)
    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      value: { getUserMedia: async () => ({ getTracks: () => [{ stop: () => {} }] }) },
    })
  })

  const record = async (): Promise<void> => {
    fireEvent.click(screen.getByLabelText('Dictate'))
    fireEvent.click(await screen.findByLabelText('Stop dictating'))
  }

  it('puts what the machine heard in the composer, for the person to send', async () => {
    const { send, say } = shown(orchestrator, { voice: true })
    await record()
    await waitFor(() => expect(sentOf(send, 'voice')).toHaveLength(1))
    const [voice] = sentOf(send, 'voice') as Extract<RemoteIn, { t: 'voice' }>[]
    expect(voice!.mime).toBe('audio/webm;codecs=opus')
    expect(screen.getByText('The machine is listening to it…')).toBeTruthy()

    fireEvent.change(screen.getByLabelText('Message'), { target: { value: 'Then' } })
    say({ t: 'heard', upload: voice!.upload, transcript: { text: 'deploy the api', note: null } })
    expect((screen.getByLabelText('Message') as HTMLTextAreaElement).value).toBe('Then deploy the api')
    // Heard is not sent: the person reads it first.
    expect(sentOf(send, 'chatSay')).toHaveLength(0)
  })

  it('refuses a recording past what the machine takes', async () => {
    recording = new Uint8Array(3 * 1024 * 1024 + 1)
    const { send } = shown(orchestrator, { voice: true })
    await record()
    await waitFor(() => expect(screen.getByText(/at most 3 MB/)).toBeTruthy())
    expect(sentOf(send, 'uploadPart')).toHaveLength(0)
    expect(sentOf(send, 'voice')).toHaveLength(0)
  })

  it('falls back to the phone’s own recognition, or offers no mic at all', () => {
    vi.stubGlobal('MediaRecorder', undefined)
    shown(orchestrator, { voice: true })
    expect(screen.queryByLabelText('Dictate')).toBeNull()
    cleanup()

    const started = vi.fn()
    class Recognition {
      onresult: ((event: unknown) => void) | null = null
      start = (): void => {
        started()
        this.onresult?.({ resultIndex: 0, results: [Object.assign([{ transcript: ' and the site ' }], { isFinal: true })] })
      }
      stop = (): void => {}
    }
    vi.stubGlobal('webkitSpeechRecognition', Recognition)
    shown(orchestrator, { voice: false })
    fireEvent.click(screen.getByLabelText('Dictate'))
    expect(started).toHaveBeenCalled()
    expect((screen.getByLabelText('Message') as HTMLTextAreaElement).value).toBe('and the site')
  })
})
