import { useEffect, useState } from 'react'

import './shell.css'
import { AddProject } from './AddProject'
import { ContextMenu } from './ContextMenu'
import { ask, commands } from './live'
import { Onboarding } from './Onboarding'
import { Palette } from './Palette'
import { Panes } from './Panes'
import { RemoveProject } from './RemoveProject'
import { ResizeEdges } from './ResizeEdges'
import { RightPanel } from './RightPanel'
import { Settings } from './Settings'
import { Sidebar } from './Sidebar'
import { SignIn } from './SignIn'
import { TopBar } from './TopBar'
import { ShellProvider, useShell } from './useShell'
import { isMaximized, onResized } from './window'

export function AppShell(): React.JSX.Element {
  return (
    <ShellProvider>
      <Window />
    </ShellProvider>
  )
}

/*
 * Three columns under one bar. The bar spans the whole width because the
 * window is one project, and the project is not a property of a column.
 *
 * A closed panel is a zero-width column rather than an unmounted one: the
 * terminal keeps its size and its scrollback instead of being rebuilt every
 * time the tree is hidden.
 */
function Window(): React.JSX.Element {
  const shell = useShell()
  const { side, files, signedIn, prefs, projects, forgetProject } = shell
  const [palette, setPalette] = useState(false)
  const [signIn, setSignIn] = useState(false)
  const [adding, setAdding] = useState(false)
  const [removing, setRemoving] = useState<string | null>(null)
  /* Opening a file is the same act wherever it was clicked: the tree on the
     right, the Files pane, or the palette. The id is the path, so the same
     file twice lands on the tab you already have. */
  const openInTab = (path: string): void =>
    shell.show('file', { id: `file:${path}`, path, title: path.split('/').pop() })
  const [maximized, setMaximized] = useState(false)

  /* Maximised, the rounded corners square off — a rounded rectangle floating
     in the middle of a screen it should fill reads as a bug. */
  useEffect(() => {
    const check = (): void => void isMaximized().then(setMaximized)
    check()
    return onResized(check)
  }, [])

  /* The account state is read by CSS, which decides which half of the
     sidebar foot exists. */
  useEffect(() => {
    document.body.dataset.account = signedIn ? 'in' : 'out'
  }, [signedIn])

  useEffect(() => {
    document.documentElement.dataset.theme = shell.theme
  }, [shell.theme])

  useEffect(() => {
    const key = (event: KeyboardEvent): void => {
      const meta = event.metaKey || event.ctrlKey
      if (meta && event.key.toLowerCase() === 'k') {
        event.preventDefault()
        setPalette((was) => !was)
      }
      if (meta && event.key.toLowerCase() === 't') {
        event.preventDefault()
        shell.show('term')
      }
      if (meta && event.key.toLowerCase() === 'n') {
        event.preventDefault()
        shell.show('chat')
      }
      if (event.key === 'Escape') {
        setPalette(false)
        setSignIn(false)
        setAdding(false)
        setRemoving(null)
        shell.closePrefs()
      }
    }
    document.addEventListener('keydown', key)
    return () => document.removeEventListener('keydown', key)
  }, [shell])

  return (
    <div className="app" data-max={String(maximized)}>
      <ResizeEdges />
      <ContextMenu />
      <TopBar onAddProject={() => setAdding(true)} />

      <div className="win" data-side={side ? 'open' : 'closed'} data-files={files ? 'open' : 'closed'}>
        <Sidebar onSearch={() => setPalette(true)} onSignIn={() => setSignIn(true)} />
        <Panes onOpenFile={openInTab} />
        <RightPanel onOpenFile={openInTab} />
      </div>

      {/* No project, nothing to show: the setup screen is the empty state. */}
      {projects.length === 0 && (
        <Onboarding
          onAddProject={() => setAdding(true)}
          onDone={() => void ask(() => commands.settingsFinishOnboarding())}
        />
      )}

      {palette && <Palette onClose={() => setPalette(false)} />}
      {signIn && (
        <SignIn
          onClose={() => setSignIn(false)}
          onSignIn={() => {
            shell.signIn()
            setSignIn(false)
          }}
        />
      )}
      {adding && <AddProject onClose={() => setAdding(false)} />}
      {removing && (
        <RemoveProject
          project={removing}
          onClose={() => setRemoving(null)}
          onConfirm={() => {
            forgetProject(removing)
            setRemoving(null)
          }}
        />
      )}
      {prefs && (
        <Settings
          pane={prefs}
          onAddProject={() => setAdding(true)}
          onRemove={(project) => setRemoving(project)}
        />
      )}
    </div>
  )
}
