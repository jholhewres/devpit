import type { Project, Worktree } from '../gen/bindings'

/**
 * The surfaces a project can open.
 *
 * Not in the contract: which drawings exist is a question for the screen, and
 * a backend answering a question nobody asks is the same defect as a button
 * that does nothing, read from the other side.
 */
export type SurfaceId = 'overview' | 'canvas' | 'notes' | 'wiki' | 'diagnostics'

/** The checkout a project opens into, and the one its chrome describes. */
export function currentWorktree(project: Project): Worktree | null {
  return project.worktrees.find((worktree) => worktree.current) ?? project.worktrees[0] ?? null
}
