import { act, cleanup, render, renderHook, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { EndedSession, LiveSession } from '../gen/bindings'
import { endedLatestFirst, needingYou, roundOf, toResume } from './orchestratorShortcuts'
import { ORCHESTRATOR_OWN, ownAsked, ownFor } from './slash'

const live = (name: string, extra: Partial<LiveSession> = {}): LiveSession =>
  ({ name, pid: 1, status: 'idle', kind: 'interactive', cwd: '/w', projectId: 'api', projectName: 'api', cardId: null, since: null, inDevpit: true, waiting: null, pane: null, step: null, draft: null, ...extra }) as LiveSession

const ended = (name: string, extra: Partial<EndedSession> = {}): EndedSession => ({
  sessionId: `id-${name}`,
  name,
  cwd: '/w/api',
  projectId: 'api',
  projectName: 'api',
  cardId: null,
  startedAt: 1,
  endedAt: 2,
  endedBy: null,
  lastStatus: 'idle',
  dirty: null,
  ...extra,
})

const asking = { question: 'Run the migration?', options: [{ label: 'Yes', hint: null }], cursor: 0 }

const running = vi.fn((): LiveSession[] => [])
const gone = vi.fn((): EndedSession[] => [])
const resumed = vi.fn()

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    chatSlashCommands: () => ['compact', 'status'],
    orchestratorSessions: () => ({ sessions: running() }),
    orchestratorEnded: () => ({ sessions: gone() }),
    orchestratorResume: (...args: unknown[]) => (resumed(...args), 'fix-login'),
  },
}))

const { useSlash } = await import('./useSlash')
const { OrchestratorAnswer } = await import('./OrchestratorAnswer')

beforeEach(() => {
  running.mockReset().mockReturnValue([])
  gone.mockReset().mockReturnValue([])
  resumed.mockReset()
})
afterEach(cleanup)

describe('what an orchestrator chat reads as a shortcut', () => {
  const mine = ownFor(true)

  it('is one of its four words, alone', () => {
    expect(ownAsked('/sessions', mine)).toEqual({ name: 'sessions', arg: '' })
    expect(ownAsked('  /waiting ', mine)).toEqual({ name: 'waiting', arg: '' })
    expect(ownAsked('/ended', mine)).toEqual({ name: 'ended', arg: '' })
    expect(ownAsked('/resume-session', mine)).toEqual({ name: 'resume-session', arg: '' })
  })

  it('takes words after /resume-session only: after another, the words are a message', () => {
    expect(ownAsked('/resume-session fix-login', mine)).toEqual({ name: 'resume-session', arg: 'fix-login' })
    expect(ownAsked('/sessions of the deploy?', mine)).toBeNull()
  })

  it('is nothing in another chat, nor anything that is not one', () => {
    expect(ownAsked('/sessions', ownFor(false))).toBeNull()
    expect(ownAsked('sessions', mine)).toBeNull()
    expect(ownAsked('/compact', mine)).toBeNull()
    // Claude Code's own `/status` is left to the CLI.
    expect(ownAsked('/status', mine)).toBeNull()
    expect(ownFor(false)).not.toContain('resume-session')
    expect(mine).toEqual(expect.arrayContaining([...ORCHESTRATOR_OWN]))
  })
})

describe('an orchestrator chat’s composer', () => {
  it('offers the shortcuts before the CLI’s commands, hiding none of them', async () => {
    const { result } = renderHook(() => useSlash('claude', '/', vi.fn(), true))
    await waitFor(() => expect(result.current.items).toEqual(['resume', 'sessions', 'waiting', 'ended', 'resume-session', 'compact', 'status']))
  })

  it('opens a shortcut picked from the menu instead of typing it', async () => {
    const setPrompt = vi.fn()
    const { result } = renderHook(() => useSlash('claude', '/wait', setPrompt, true))
    await waitFor(() => expect(result.current.items).toEqual(['waiting']))
    act(() => result.current.choose('waiting'))
    expect(setPrompt).toHaveBeenCalledWith('')
    expect(result.current.own).toEqual({ name: 'waiting', arg: '' })
  })

  it('runs one sent as typed, with its words, and lets anything else through', () => {
    const { result } = renderHook(() => useSlash('claude', '', vi.fn(), true))
    let took = false
    act(() => {
      took = result.current.ran('/resume-session fix-login')
    })
    expect(took).toBe(true)
    expect(result.current.own).toEqual({ name: 'resume-session', arg: 'fix-login' })
    expect(result.current.ran('what is running?')).toBe(false)
  })

  it('leaves the words to the agent in a chat that is not an orchestrator’s', () => {
    const { result } = renderHook(() => useSlash('claude', '', vi.fn()))
    expect(result.current.ran('/sessions')).toBe(false)
  })
})

describe('what each shortcut selects', () => {
  it('/waiting: a question or a draft waiting on the person, and nothing else', () => {
    const sessions = [live('idle-one'), live('asks', { waiting: asking }), live('drafted', { draft: 'go on' }), live('busy-one', { status: 'busy' })]
    expect(needingYou(sessions).map((one) => one.name)).toEqual(['asks', 'drafted'])
  })

  it('/sessions: every running session once, by what it is doing, and the latest ended', () => {
    const sessions = [live('idle-one'), live('asks', { status: 'busy', waiting: asking }), live('busy-one', { status: 'busy' })]
    const endedOnes = [ended('old', { endedAt: 1 }), ended('a'), ended('b', { endedAt: 5 }), ended('c', { endedAt: 4 })]
    const round = roundOf(sessions, endedOnes)
    expect(round.waiting.map((one) => one.name)).toEqual(['asks'])
    expect(round.working.map((one) => one.name)).toEqual(['busy-one'])
    expect(round.idle.map((one) => one.name)).toEqual(['idle-one'])
    expect(round.ended.map((one) => one.name)).toEqual(['b', 'c', 'a'])
  })

  it('/ended: the sessions that ended, the latest first', () => {
    expect(endedLatestFirst([ended('a', { endedAt: 1 }), ended('b', { endedAt: 3 }), ended('c', { endedAt: null })]).map((one) => one.name)).toEqual(['b', 'a', 'c'])
  })

  it('/resume-session: an id, else the latest under a name, else every name containing the words', () => {
    const list = [ended('fix-login', { sessionId: 'aaa', endedAt: 1 }), ended('fix-login', { sessionId: 'bbb', endedAt: 9 }), ended('fix-logout', { sessionId: 'ccc' }), ended('nowhere', { projectId: null })]
    expect(toResume(list, 'aaa').map((one) => one.sessionId)).toEqual(['aaa'])
    expect(toResume(list, 'FIX-LOGIN').map((one) => one.sessionId)).toEqual(['bbb'])
    // The latest first: `ccc` ended at 2, after `aaa`.
    expect(toResume(list, 'fix').map((one) => one.sessionId)).toEqual(['bbb', 'ccc', 'aaa'])
    // One with no project cannot reopen in a terminal of one.
    expect(toResume(list, '').map((one) => one.name)).not.toContain('nowhere')
    expect(toResume(list, 'nowhere')).toEqual([])
  })
})

describe('a shortcut’s answer', () => {
  it('/sessions draws the round from devpit’s own reading', async () => {
    running.mockReturnValue([live('asks', { waiting: asking }), live('busy-one', { status: 'busy' })])
    gone.mockReturnValue([ended('done-one')])
    render(<OrchestratorAnswer asked={{ name: 'sessions', arg: '' }} profileId="claude" onClose={vi.fn()} />)
    expect(await screen.findByText('Waiting on you · 1')).toBeTruthy()
    expect(screen.getByText('Run the migration?')).toBeTruthy()
    expect(screen.getByText('Working · 1')).toBeTruthy()
    expect(await screen.findByText('done-one')).toBeTruthy()
  })

  it('/resume-session naming one session resumes it, once', async () => {
    gone.mockReturnValue([ended('fix-login'), ended('other')])
    render(<OrchestratorAnswer asked={{ name: 'resume-session', arg: 'fix-login' }} profileId="claude" onClose={vi.fn()} />)
    expect(await screen.findByText(/Resumed as fix-login/)).toBeTruthy()
    expect(resumed).toHaveBeenCalledTimes(1)
    expect(resumed).toHaveBeenCalledWith('claude', 'id-fix-login')
  })

  it('/resume-session naming several lets the person pick', async () => {
    gone.mockReturnValue([ended('fix-login'), ended('fix-logout')])
    render(<OrchestratorAnswer asked={{ name: 'resume-session', arg: 'fix' }} profileId="claude" onClose={vi.fn()} />)
    expect(await screen.findAllByRole('button', { name: 'Resume' })).toHaveLength(2)
    expect(resumed).not.toHaveBeenCalled()
  })
})
