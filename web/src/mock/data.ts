/*
 * Everything the window shows before anything is wired.
 *
 * One module, on purpose: the step that connects the backend is then a
 * subtraction rather than a hunt. What is still imported from here when that
 * step ends is a gap the backend did not fill, not a mock someone forgot.
 */

export interface MockProject {
  readonly name: string
  readonly path: string
  readonly live: number
  readonly seen: string
}

export const PROJECTS: readonly MockProject[] = [
  { name: 'devpit', path: '~/Workspace/private/devpit', live: 2, seen: 'open now' },
  { name: 'orca', path: '~/Workspace/private/sources/orca', live: 0, seen: 'yesterday' },
  { name: 'anchored', path: '~/Workspace/private/anchored', live: 1, seen: '2 days ago' },
  { name: 'waku', path: '~/Workspace/private/sources/waku', live: 0, seen: '5 days ago' },
  { name: 'hg-portal', path: '~/HostGator/portal', live: 0, seen: '3 weeks ago' },
]

export const BRANCH = { name: 'main', ahead: 2, added: 287, deleted: 195 } as const

export const ACCOUNT = {
  name: 'Jhol Hewres',
  email: 'jhol.code@gmail.com',
  initials: 'JH',
} as const
