import { AskName } from './AskName'
import { Confirm } from './Confirm'
import type { FileActions } from './useFileActions'

/* The three dialogs the file actions raise. Given the whole `FileActions`
   rather than eight props: they are one thing, and a caller that has the hook
   has all of it already. */
export function FileDialogs({ actions }: { actions: FileActions }): React.JSX.Element | null {
  const { naming, renaming, deleting, close, create, rename, destroy } = actions

  if (naming) {
    return (
      <AskName
        title={naming.folder ? 'New folder' : 'New file'}
        placeholder={naming.folder ? 'components' : 'thing.ts'}
        action="Create"
        onClose={close}
        onName={create}
      />
    )
  }

  if (renaming) {
    const name = renaming.split('/').pop() ?? ''
    return (
      <AskName
        title={`Rename ${name}`}
        placeholder={name}
        action="Rename"
        onClose={close}
        onName={rename}
      />
    )
  }

  if (deleting) {
    return (
      <Confirm
        title={`Delete ${deleting}?`}
        danger="Delete"
        body={
          <>
            This removes it from the folder. Git can bring back anything it has already committed;{' '}
            <b>anything it has not is gone.</b>
          </>
        }
        onClose={close}
        onConfirm={destroy}
      />
    )
  }

  return null
}
