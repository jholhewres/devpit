import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Project, ProjectProposal } from '../gen/bindings'
import { applied, proposalWords } from './projectProposal'

const calls: string[] = []
let links: string[] = []
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: {
    projectAdd: (path: string) => (calls.push(`add ${path}`), { id: 'prj_new', name: 'asc-api', group: null, icon: null, color: null }),
    projectEdit: (id: string, name: string, group: string | null) => (calls.push(`edit ${id} ${name} ${group}`), {}),
    orchestratorLinks: () => links,
    orchestratorLink: (_: string, linked: string[]) => (calls.push(`link ${linked.join(',')}`), null),
  },
}))

afterEach(() => {
  calls.length = 0
  links = []
})

const api = { id: 'prj_api', name: 'api', group: null, icon: null, color: null } as unknown as Project
const proposal = (over: Partial<ProjectProposal>): ProjectProposal => ({ id: 'p1', projectId: null, path: '/w/asc-api', name: 'asc-api', group: null, link: true, ...over })

describe('a proposed project change', () => {
  it('is said in words', () => {
    expect(proposalWords(proposal({ group: 'ASC' }), [])).toBe('Add /w/asc-api as asc-api, put it in ASC, link it here')
    expect(proposalWords(proposal({ projectId: 'prj_api', name: 'api' }), [api])).toBe('Link api here')
    expect(proposalWords(proposal({ projectId: 'prj_api', name: 'api', link: false }), [api])).toBe('Unlink api')
    expect(proposalWords(proposal({ projectId: 'prj_api', name: 'api', group: 'ASC', link: null }), [api])).toBe('Put api in ASC')
  })

  it('adds the folder, groups it and links it, keeping the other links', async () => {
    links = ['prj_api']
    expect(await applied(proposal({ group: 'ASC' }), 'prj_orch', [api])).toBeNull()
    expect(calls).toEqual(['add /w/asc-api', 'edit prj_new asc-api ASC', 'link prj_api,prj_new'])
  })

  it('touches only what it changes on a project devpit has', async () => {
    links = ['prj_api', 'prj_x']
    expect(await applied(proposal({ projectId: 'prj_api', name: 'api', link: false }), 'prj_orch', [api])).toBeNull()
    expect(calls).toEqual(['link prj_x'])
  })
})
