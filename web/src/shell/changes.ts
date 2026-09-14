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

/* What discarding this status actually throws away — the three cases are not
   the same loss. A deleted file comes back exactly as HEAD has it, so
   restoring it loses nothing. A modified file also comes back as HEAD has
   it, but that means every uncommitted edit in between is gone. Added and
   untracked content has no earlier version anywhere, so discarding it is the
   file itself going away. */
export type DiscardLoss = 'nothing' | 'edits' | 'file'

export const discardLoses = (status: Change['status']): DiscardLoss => {
  switch (status) {
    case 'deleted':
      return 'nothing'
    case 'modified':
      return 'edits'
    default:
      return 'file'
  }
}
