import type { Change } from '../gen/bindings'
import { discardLoses } from './changes'
import { Confirm } from './Confirm'

/*
 * The confirmation text for one discard, apart from `Changes.tsx` because the
 * three losses read very differently and getting the wrong one wrong is worse
 * than saying none: "nothing is lost" next to a modified file would throw
 * away an hour of uncommitted edits while promising it would not.
 */
const body = (status: Change['status']): React.ReactNode => {
  switch (discardLoses(status)) {
    case 'nothing':
      return 'Restores it exactly as git already has it — nothing is lost.'
    case 'edits':
      return (
        <>
          <b>Every uncommitted edit in this file is lost.</b> It goes back to what git already
          has.
        </>
      )
    case 'file':
      return (
        <>
          Git has no earlier copy of this file. <b>Discarding deletes it for good.</b>
        </>
      )
  }
}

export function DiscardConfirm({
  change,
  onClose,
  onConfirm,
}: {
  change: Change
  onClose: () => void
  onConfirm: () => void
}): React.JSX.Element {
  return (
    <Confirm
      title={`Discard ${change.path}?`}
      danger="Discard"
      body={body(change.status)}
      onClose={onClose}
      onConfirm={onConfirm}
    />
  )
}
