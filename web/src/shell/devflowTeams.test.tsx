import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { DevflowConfig, DevflowTeam } from '../gen/bindings'

const called = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    devflowTeamFetch: async (projectId: string) => {
      called('fetch', projectId)
      return {
        changes: [
          {
            name: 'acme',
            git: 'git@x:acme/flow.git',
            fromRef: 'v1',
            fromCommit: 'aaaa',
            toRef: 'v2',
            toCommit: 'bbbbbbbbbbbbbbbb',
            files: ['changed devflow.yaml', 'added skills/pr/SKILL.md'],
            diff: ['+     persona: devflow-spec'],
          },
        ],
      }
    },
    devflowTeamApprove: async (...args: unknown[]) => {
      called('approve', ...args)
      return { projectId: 'prj_api' }
    },
  },
}))

const { DevflowTeams, teamSaid } = await import('./DevflowTeams')

const team = (state: string): DevflowTeam => ({ name: 'acme', git: 'git@x:acme/flow.git', wanted: 'v2', lockedRef: 'v1', commit: 'aaaaaaaaaaaaaaaa', state })
const config = (teams: DevflowTeam[]): DevflowConfig => ({
  enabled: true,
  version: 2,
  teams,
  locked: ['review.reviewers.devflow-security'],
  stagesFrom: 'project',
  stages: [],
  reviewers: [],
  blockOn: [],
  consolidate: 'jev',
  decisionsUse: false,
  decisionsMode: 'shadow',
})

afterEach(() => {
  cleanup()
  called.mockReset()
})

describe('the layers a project extends', () => {
  it('says what applies, what waits for the person and what they locked', () => {
    expect(teamSaid(team('applied'))).toBe('v2 applies, pinned at aaaaaaaaaaaa')
    expect(teamSaid(team('approve'))).toBe('v2 waits for your approval; v1 still applies')
    render(<DevflowTeams projectId="prj_api" config={config([team('approve')])} onScreen={() => {}} />)
    expect(screen.getByText(/review\.reviewers\.devflow-security/)).toBeTruthy()
    expect(called).not.toHaveBeenCalled()
  })

  it('shows what a new ref changes, and pins it only on the approval', async () => {
    const onScreen = vi.fn()
    render(<DevflowTeams projectId="prj_api" config={config([team('approve')])} onScreen={onScreen} />)
    fireEvent.click(screen.getByRole('button', { name: 'Look at what changes' }))
    expect(await screen.findByText('added skills/pr/SKILL.md')).toBeTruthy()
    expect(screen.getByLabelText("acme's devflow.yaml change").textContent).toContain('+     persona: devflow-spec')
    expect(called).not.toHaveBeenCalledWith('approve', expect.anything(), expect.anything(), expect.anything())
    fireEvent.click(screen.getByRole('button', { name: 'Approve v2' }))
    await waitFor(() => expect(called).toHaveBeenCalledWith('approve', 'prj_api', 'git@x:acme/flow.git', 'bbbbbbbbbbbbbbbb'))
    await waitFor(() => expect(onScreen).toHaveBeenCalled())
  })

  it('is absent where nothing is extended or locked', () => {
    const { container } = render(<DevflowTeams projectId="prj_api" config={{ ...config([]), locked: [] }} onScreen={() => {}} />)
    expect(container.innerHTML).toBe('')
  })
})
