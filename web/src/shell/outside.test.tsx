import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Installation, OutsideSession, Profile, Project } from '../gen/bindings'
import { installationOf, profileFor, scopeOf, sessionInScope, threadInScope, titled } from './outside'
import { OutsideThreads } from './OutsideThreads'

afterEach(cleanup)

const profile = (over: Partial<Profile>): Profile =>
  ({ id: 'p', label: 'p', command: 'claude', driver: 'claude', path: '/bin/claude', reach: 'runnable', base: '', args: [], env: [], mine: false, models: [], efforts: [], effortDefault: null, ...over }) as Profile

const session = (over: Partial<OutsideSession> = {}): OutsideSession => ({
  sessionId: 'aaa', title: 'Fix the parser', installation: '/home/me/.claude-glm', lastAt: 0, ...over,
})

const installations: Installation[] = [
  { directory: '/home/me/.claude', profiles: [], default: true },
  { directory: '/home/me/.claude-glm', profiles: ['glm'], default: false },
]
const profiles = [profile({ id: 'claude', label: 'Claude Code' }), profile({ id: 'prof_glm', label: 'glm', mine: true })]

describe('which profile opens a session from a terminal', () => {
  it('is a profile running against the installation that wrote it', () => {
    expect(profileFor(session(), installations, profiles)?.id).toBe('prof_glm')
  })

  it('is the CLI devpit found itself, for the installation no profile names', () => {
    expect(profileFor(session({ installation: '/home/me/.claude' }), installations, profiles)?.id).toBe('claude')
  })

  it('is nobody when no runnable profile reaches that installation', () => {
    // Resuming under another installation would start with none of its history.
    const onlyShell = [profile({ id: 'prof_glm', label: 'glm', mine: true, reach: 'shell_only' })]
    expect(profileFor(session(), installations, onlyShell)).toBeNull()
    expect(profileFor(session({ installation: '/elsewhere' }), installations, profiles)).toBeNull()
  })

  it('matches by id when the answer carries ids, so a shared name is not a match', () => {
    const byId: Installation[] = [
      installations[0],
      { directory: '/home/me/.claude-glm', profiles: ['glm'], ids: ['prof_other'], default: false },
    ]
    expect(profileFor(session(), byId, profiles)).toBeNull()
  })

  it('is named by the CLI title, or says it has none', () => {
    expect(titled(session())).toBe('Fix the parser')
    expect(titled(session({ title: null }))).toBe('Untitled session')
  })
})

/* Each account keeps its own history, so the chat's earlier conversations
   narrow to the one it is on. */
describe('which earlier conversations belong to the account a chat is on', () => {
  const withFast = [...profiles, profile({ id: 'prof_fast', label: 'glm-fast', mine: true })]
  const shared: Installation[] = [
    installations[0],
    { directory: '/home/me/.claude-glm', profiles: ['glm', 'glm-fast'], ids: ['prof_glm', 'prof_fast'], default: false },
  ]

  it('finds where a profile runs: its own directory, or this process’s for one devpit found', () => {
    expect(installationOf(profiles[1], installations)?.directory).toBe('/home/me/.claude-glm')
    expect(installationOf(profiles[0], installations)?.directory).toBe('/home/me/.claude')
    expect(installationOf(profile({ id: 'x', mine: true }), installations)).toBeNull()
    expect(installationOf(profile({ id: 'c', driver: 'codex' }), installations)).toBeNull()
  })

  it('keeps terminal sessions its installation wrote, and only those', () => {
    const glm = scopeOf('prof_glm', profiles, installations)
    expect(sessionInScope('/home/me/.claude-glm', glm)).toBe(true)
    expect(sessionInScope('/home/me/.claude', glm)).toBe(false)
  })

  it('keeps its own chats, and those of a profile on the same installation', () => {
    const glm = scopeOf('prof_glm', withFast, shared)
    expect(threadInScope('prof_glm', glm, withFast, shared)).toBe(true)
    expect(threadInScope('prof_fast', glm, withFast, shared)).toBe(true)
    expect(threadInScope('claude', glm, withFast, shared)).toBe(false)
  })

  it('narrows nothing before an account is chosen', () => {
    const none = scopeOf(null, profiles, installations)
    expect(none).toBeNull()
    expect(sessionInScope('/anywhere', none)).toBe(true)
    expect(threadInScope('claude', none, profiles, installations)).toBe(true)
  })
})

const adopted = vi.fn()
const show = vi.fn()
const project = { id: 'prj_1', name: 'demos' } as unknown as Project
vi.mock('./useShell', () => ({ useShell: () => ({ project, show }) }))
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: {
    chatOutside: () => [session()],
    cliInstallations: () => installations,
    agentProfiles: () => profiles,
    chatAdopt: (projectId: string, sessionId: string, profileId: string, title: string | null) => {
      adopted(projectId, sessionId, profileId, title)
      return 'conv_new'
    },
  },
}))

describe('the list of sessions from a terminal', () => {
  it('lists them under their CLI title and opens one as a conversation that resumes it', async () => {
    render(<OutsideThreads />)
    fireEvent.click(await screen.findByText('Fix the parser'))
    await waitFor(() => expect(adopted).toHaveBeenCalledWith('prj_1', 'aaa', 'prof_glm', 'Fix the parser'))
    expect(show).toHaveBeenCalledWith('chat', { id: 'conv_new', title: 'Fix the parser' })
  })
})
