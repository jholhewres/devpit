import type { Change } from '../gen/bindings'
import { discardLoses, type DiscardLoss } from './changes'
import { Confirm } from './Confirm'

/*
 * The confirmation text for one discard, apart from `Changes.tsx` because the
 * three losses read very differently and getting the wrong one wrong is worse
 * than saying none: "nothing is lost" next to a modified file would throw
 * away an hour of uncommitted edits while promising it would not.
 */
const body = (worst: DiscardLoss, many: boolean): React.ReactNode => {
  switch (worst) {
    case 'nothing':
      return many
        ? 'Restores them exactly as git already has them — nothing is lost.'
        : 'Restores it exactly as git already has it — nothing is lost.'
    case 'edits':
      return (
        <>
          <b>Every uncommitted edit in {many ? 'these files' : 'this file'} is lost.</b>{' '}
          {many ? 'They go' : 'It goes'} back to what git already has.
        </>
      )
    case 'file':
      return (
        <>
          Git has no earlier copy of {many ? 'some of these files' : 'this file'}.{' '}
          <b>Discarding deletes {many ? 'those' : 'it'} for good.</b>
        </>
      )
  }
}

/* Several at once are described by the worst of them: a section holding one
   new file among twenty edits deletes that file, and saying only "edits are
   lost" would hide it. */
const WORST: readonly DiscardLoss[] = ['file', 'edits', 'nothing']

const loss = (changes: readonly Change[]): DiscardLoss =>
  WORST.find((one) => changes.some((change) => discardLoses(change.status) === one)) ?? 'nothing'

export function DiscardConfirm({
  changes,
  onClose,
  onConfirm,
}: {
  changes: readonly Change[]
  onClose: () => void
  onConfirm: () => void
}): React.JSX.Element {
  const one = changes.length === 1 ? changes[0] : undefined
  return (
    <Confirm
      title={one ? `Discard ${one.path}?` : `Discard ${changes.length} files?`}
      danger="Discard"
      body={body(loss(changes), one === undefined)}
      onClose={onClose}
      onConfirm={onConfirm}
    />
  )
}
