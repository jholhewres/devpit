import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { LiveSession } from '../gen/bindings'
import { ReplyDrafts } from './ReplyDrafts'

const replied = vi.fn(async (..._args: unknown[]) => null as unknown)
const dropped = vi.fn(async (..._args: unknown[]) => undefined)
let refusal: string | null = null
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => {
    const data = await call()
    return { data, error: refusal, loading: false }
  },
  commands: {
    orchestratorReply: (...args: unknown[]) => replied(...args),
    orchestratorDraftDrop: (...args: unknown[]) => dropped(...args),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  refusal = null
})

const live = (name: string, extra: Partial<LiveSession> = {}): LiveSession =>
  ({
    name,
    status: 'idle',
    kind: 'interactive',
    cwd: '/w',
    projectId: 'p',
    projectName: 'api',
    cardId: null,
    since: null,
    inDevpit: true,
    waiting: null,
    pane: { projectId: 'p', paneId: 'leaf_1' },
    draft: null,
    ...extra,
  }) as LiveSession

describe('a reply the orchestrator drafted', () => {
  it('shows nothing when nothing was drafted', () => {
    const { container } = render(<ReplyDrafts profileId="claude" sessions={[live('api-a')]} onDone={vi.fn()} />)
    expect(container.innerHTML).toBe('')
  })

  it('is sent only by the click, as the person typed it last', async () => {
    const onDone = vi.fn()
    render(<ReplyDrafts profileId="claude" sessions={[live('api-a', { draft: 'yes, delete the folder' })]} onDone={onDone} />)
    const text = screen.getByLabelText('Reply to api-a, as you')
    expect((text as HTMLTextAreaElement).value).toBe('yes, delete the folder')
    expect(replied).not.toHaveBeenCalled()
    fireEvent.change(text, { target: { value: 'yes, delete it, keep the rollback' } })
    fireEvent.click(screen.getByRole('button', { name: 'Send as you' }))
    await waitFor(() => expect(replied).toHaveBeenCalledWith('claude', 'api-a', 'yes, delete it, keep the rollback'))
    await waitFor(() => expect(onDone).toHaveBeenCalled())
  })

  it('is dropped without sending anything', async () => {
    render(<ReplyDrafts profileId="claude" sessions={[live('api-a', { draft: 'go on' })]} onDone={vi.fn()} />)
    fireEvent.click(screen.getByRole('button', { name: 'Drop' }))
    await waitFor(() => expect(dropped).toHaveBeenCalledWith('claude', 'api-a'))
    expect(replied).not.toHaveBeenCalled()
  })

  it('waits while the session is stopped on a question, which hears nothing typed', () => {
    render(
      <ReplyDrafts
        profileId="claude"
        sessions={[live('api-a', { draft: 'go on', waiting: { question: 'Proceed?', options: [], cursor: 0 } as LiveSession['waiting'] })]}
        onDone={vi.fn()}
      />,
    )
    expect((screen.getByRole('button', { name: 'Send as you' }) as HTMLButtonElement).disabled).toBe(true)
    expect(screen.getByText(/answer that first/)).toBeTruthy()
  })

  it('says why when the terminal refused it', async () => {
    refusal = 'the agent is not in front in that terminal — open it to see why'
    render(<ReplyDrafts profileId="claude" sessions={[live('api-a', { draft: 'go on' })]} onDone={vi.fn()} />)
    fireEvent.click(screen.getByRole('button', { name: 'Send as you' }))
    await waitFor(() => screen.getByText(/not in front in that terminal/))
  })
})
