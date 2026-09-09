import type { Project } from '../gen/bindings'

/* What removing a project does to the list and to where you are standing.
   A function, so the test calls the rule instead of restating it. */
export interface Open {
  readonly projects: readonly Project[]
  readonly current: string | null
}

export function forgotten(open: Open, id: string): Open {
  const projects = open.projects.filter((other) => other.id !== id)
  return {
    projects,
    /* Removing the ground you stand on has to land somewhere, or the window
       keeps naming a project that is no longer listed. */
    current: open.current === id ? (projects[0]?.id ?? null) : open.current,
  }
}

export const found = (open: Open): Project | null =>
  open.projects.find((project) => project.id === open.current) ?? null
