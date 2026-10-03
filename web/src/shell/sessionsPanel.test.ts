import { describe, expect, it, vi } from 'vitest'

import type { LiveSession, Project } from '../gen/bindings'
import { panelGroups, stateOfOne } from './sessionsPanel'

vi.mock('./live', () => ({ ask: vi.fn(), commands: {} }))

const live = (name: string, projectId: string | null, extra: Partial<LiveSession> = {}): LiveSession =>
  ({ name, pid: 1, status: 'idle', kind: 'interactive', cwd: '/w', projectId, projectName: projectId, cardId: null, since: null, inDevpit: true, waiting: null, pane: null, step: null, ...extra }) as LiveSession

const project = (id: string): Project => ({ id, name: id, rootPath: `/w/${id}` }) as Project

const projects = ['api', 'web', 'docs'].map(project)
const ids = (groups: ReturnType<typeof panelGroups>): (string | null)[] => groups.map((one) => one.project?.id ?? null)

describe('the Sessions panel', () => {
  it('gives a group only to a project with something running, unless asked', () => {
    const sessions = [live('api-1', 'api')]
    expect(ids(panelGroups(sessions, ['api', 'docs'], projects, '', false))).toEqual(['api'])
    expect(ids(panelGroups(sessions, ['api', 'docs'], projects, '', true))).toEqual(['api', 'docs'])
  })

  it('says which projects the orchestrator reaches', () => {
    const groups = panelGroups([live('api-1', 'api'), live('web-1', 'web')], ['api'], projects, '', false)
    expect(groups.map((one) => [one.project?.id, one.linked])).toEqual([
      ['api', true],
      ['web', false],
    ])
  })

  it('keeps what a search names, by session, project or card', () => {
    const sessions = [live('api-1', 'api', { cardId: 'card_1' }), live('web-1', 'web')]
    const titles = new Map([['card_1', 'Fix the invoice total']])
    expect(ids(panelGroups(sessions, [], projects, 'invoice', false, titles))).toEqual(['api'])
    expect(ids(panelGroups(sessions, [], projects, 'WEB', false, titles))).toEqual(['web'])
    // A search shows what matches, never the empty groups beside it.
    expect(ids(panelGroups(sessions, ['docs'], projects, 'api', true, titles))).toEqual(['api'])
  })

  it('reads two sessions with one name as two states', () => {
    const asking = live('worker', 'api', { waiting: { question: 'Go on?', options: [] } as never })
    const idle = live('worker', 'web')
    expect([stateOfOne(asking), stateOfOne(idle)]).toEqual(['waiting', 'idle'])
  })
})
