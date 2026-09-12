import { useEffect, useState } from 'react'

import './shell.css'
import { ContextMenu } from './ContextMenu'
import { ask, commands } from './live'
import { Onboarding } from './Onboarding'
import { Overlays } from './Overlays'
import { Panes } from './Panes'
import { ResizeEdges } from './ResizeEdges'
import { RightPanel } from './RightPanel'
import { Sidebar } from './Sidebar'
import { TopBar } from './TopBar'
import { ShellProvider, useShell } from './useShell'
import { isMaximized, onResized } from './window'
import { abandoned } from './typing'

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
  const { side, files, signedIn, projects, palette, openPalette, closePalette } = shell
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
        if (palette) closePalette()
        else openPalette()
      }
      if (meta && event.key.toLowerCase() === 't') {
        event.preventDefault()
        shell.show('term')
      }
      if (meta && event.key.toLowerCase() === 'n') {
        event.preventDefault()
        shell.show('chat')
      }
      if (abandoned(event)) {
        closePalette()
        setSignIn(false)
        setAdding(false)
        setRemoving(null)
        shell.closePrefs()
      }
    }
    document.addEventListener('keydown', key)
    return () => document.removeEventListener('keydown', key)
  }, [shell, palette, openPalette, closePalette])

  return (
    <div className="app" data-max={String(maximized)}>
      <ResizeEdges />
      <ContextMenu />
      <TopBar onAddProject={() => setAdding(true)} />

      <div className="win" data-side={side ? 'open' : 'closed'} data-files={files ? 'open' : 'closed'}>
        <Sidebar onSearch={openPalette} onSignIn={() => setSignIn(true)} />
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

      <Overlays
        signIn={signIn}
        onSignInClose={() => setSignIn(false)}
        adding={adding}
        onAddingClose={() => setAdding(false)}
        onAdd={() => setAdding(true)}
        removing={removing}
        onRemove={setRemoving}
        onRemovingClose={() => setRemoving(null)}
      />
    </div>
  )
}
