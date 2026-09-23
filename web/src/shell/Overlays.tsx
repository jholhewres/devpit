import { lazy, Suspense, useEffect } from 'react'

import { AddProject } from './AddProject'
import { ask, commands } from './live'
import { Problem } from './Problem'
import { RemoveProject } from './RemoveProject'
import { ManagerPane } from './ManagerPane'
import { SignIn } from './SignIn'
import { StopRunning } from './StopRunning'
import { UpdateCard } from './UpdateCard'
import { useShell } from './useShell'

/* Opened by hand, or only while there is no project: none belongs in the
   chunk every launch parses. Each has its own boundary, so one loading does
   not hide the others; a local chunk takes a frame, so the fallback is none. */
const Onboarding = lazy(() => import('./Onboarding').then((module) => ({ default: module.Onboarding })))
const loadPalette = (): Promise<typeof import('./Palette')> => import('./Palette')
const Palette = lazy(() => loadPalette().then((module) => ({ default: module.Palette })))
const Settings = lazy(() => import('./Settings').then((module) => ({ default: module.Settings })))

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
  const { palette, closePalette, closing, prefs, forgetProject, projects } = shell
  /* The palette is fetched once the window is idle, not at Ctrl+K: until it
     is there nothing takes the focus, and the keys typed after the shortcut
     went to the terminal. */
  useEffect(() => {
    const idle = window.requestIdleCallback ?? ((run: () => void) => window.setTimeout(run, 1))
    idle(() => void loadPalette())
  }, [])

  return (
    <>
      {/* No project, nothing to show: the setup screen is the empty state. */}
      {projects.length === 0 && (
        <Suspense fallback={null}>
          <Onboarding
            onAddProject={onAdd}
            onDone={() => void ask(() => commands.settingsFinishOnboarding())}
          />
        </Suspense>
      )}
      <UpdateCard />
      <Problem />
      {palette && (
        <Suspense fallback={null}>
          <Palette onClose={closePalette} />
        </Suspense>
      )}
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
      {prefs && (
        <Suspense fallback={null}>
          <Settings pane={prefs} onAddProject={onAdd} onRemove={onRemove} />
        </Suspense>
      )}
    </>
  )
}
