/*
 * Everything the window shows before anything is wired.
 *
 * One module, on purpose: the step that connects the backend is then a
 * subtraction rather than a hunt. What is still imported from here when that
 * step ends is a gap the backend did not fill, not a mock someone forgot.
 *
 * Gone so far: PROJECTS (project.list answers for it).
 */

export const BRANCH = { name: 'main', ahead: 2, added: 287, deleted: 195 } as const

export const ACCOUNT = {
  name: 'Jhol Hewres',
  email: 'jhol.code@gmail.com',
  initials: 'JH',
} as const
