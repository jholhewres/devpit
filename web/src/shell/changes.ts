import type { Change } from '../gen/bindings'

/*
 * How the Changes panel is grouped.
 *
 * Three groups, because a commit takes one of them and not the others:
 * staged is what would go in, changed is what would not, and untracked is
 * what git is not watching at all. Collapsing untracked into changed is how
 * a new file gets left out of a commit nobody noticed was missing it.
 */

export interface Groups {
  readonly staged: readonly Change[]
  readonly changed: readonly Change[]
  readonly untracked: readonly Change[]
}

export function grouped(changes: readonly Change[]): Groups {
  return {
    staged: changes.filter((change) => change.staged),
    changed: changes.filter((change) => !change.staged && change.status !== 'untracked'),
    untracked: changes.filter((change) => !change.staged && change.status === 'untracked'),
  }
}

/* What `Stage All` would take: everything not already in the index. */
export const stageable = (changes: readonly Change[]): string[] =>
  changes.filter((change) => !change.staged).map((change) => change.path)

/* Whether committing does anything. Nothing staged means nothing to commit,
   and a commit with an empty message is refused by the backend anyway. */
export const committable = (changes: readonly Change[], message: string): boolean =>
  changes.some((change) => change.staged) && message.trim().length > 0
