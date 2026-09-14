import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Installation, OutsideSession, Profile, Project } from '../gen/bindings'
import { profileFor, titled } from './outside'
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

  it('is named by the CLI title, or says it has none', () => {
    expect(titled(session())).toBe('Fix the parser')
    expect(titled(session({ title: null }))).toBe('Untitled session')
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
