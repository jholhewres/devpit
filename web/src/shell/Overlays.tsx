import { AddProject } from './AddProject'
import { Palette } from './Palette'
import { RemoveProject } from './RemoveProject'
import { ManagerPane } from './ManagerPane'
import { Settings } from './Settings'
import { SignIn } from './SignIn'
import { StopRunning } from './StopRunning'
import { UpdateCard } from './UpdateCard'
import { useShell } from './useShell'

/*
 * Everything that opens over the window.
 *
 * Apart from the window because they are a stack, not a layout: each one is
 * present or absent, none of them moves anything else, and a seventh would
 * otherwise be a seventh thing between the layout and the eye reading it.
 */

export function Overlays({
  signIn,
  onSignInClose,
  adding,
  onAddingClose,
  onAdd,
  removing,
  onRemove,
  onRemovingClose,
}: {
  signIn: boolean
  onSignInClose: () => void
  adding: boolean
  onAddingClose: () => void
  onAdd: () => void
  removing: string | null
  onRemove: (project: string) => void
  onRemovingClose: () => void
}): React.JSX.Element {
  const shell = useShell()
  const { palette, closePalette, closing, prefs, forgetProject } = shell

  return (
    <>
      <UpdateCard />
      {palette && <Palette onClose={closePalette} />}
      {closing && (
        <StopRunning
          closing={closing}
          onCancel={shell.cancelClose}
          onConfirm={shell.confirmClose}
        />
      )}
      {signIn && <SignIn onClose={onSignInClose} />}
      {adding && <AddProject onClose={onAddingClose} />}
      {removing && (
        <RemoveProject
          project={removing}
          onClose={onRemovingClose}
          onConfirm={(wipe) => {
            forgetProject(removing, wipe)
            onRemovingClose()
          }}
        />
      )}
      {shell.managing && <ManagerPane />}
      {prefs && <Settings pane={prefs} onAddProject={onAdd} onRemove={onRemove} />}
    </>
  )
}
