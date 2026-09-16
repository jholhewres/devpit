import { useEffect, useState } from 'react'

import './shell.css'
import { ContextMenu } from './ContextMenu'
import { ask, commands } from './live'
import { Grips } from './Grips'
import { Onboarding } from './Onboarding'
import { Overlays } from './Overlays'
import { Panes } from './Panes'
import { ResizeEdges } from './ResizeEdges'
import { RightPanel } from './RightPanel'
import { Sidebar } from './Sidebar'
import { StatusStrip } from './StatusStrip'
import { TopBar } from './TopBar'
import { PluginsProvider } from './usePlugins'
import { ShellProvider, useShell } from './useShell'
import { shortcutFor } from './shortcuts'
import { isMaximized, onResized } from './window'
import { abandoned } from './typing'

export function AppShell(): React.JSX.Element {
  return (
    <ShellProvider>
      <PluginsProvider>
        <Window />
      </PluginsProvider>
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
  const { side, files, widths, signedIn, projects, palette, openPalette, closePalette } = shell
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
      // Only the window's own. A pane's keys are the pane's: splitting
      // belongs to the terminal being split, ⌘P to the picker it opens.
      const press = shortcutFor(event)
      if (press === 'palette') {
        event.preventDefault()
        if (palette) closePalette()
        else openPalette()
      }
      if (press === 'terminal' || press === 'chat') {
        event.preventDefault()
        shell.show(press === 'terminal' ? 'term' : 'chat')
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
    <div
      className="app"
      data-max={String(maximized)}
      /* On the shell and not on the grid: the top bar's lead is the same
         width as the sidebar, and it is not inside the grid. One variable,
         two readers. */
      style={
        {
          '--sidebar-w': `${widths.sidebar}px`,
          '--files-w': `${widths.files}px`,
        } as React.CSSProperties
      }
    >
      <ResizeEdges />
      <ContextMenu />
      <TopBar onAddProject={() => setAdding(true)} />

      <div className="win" data-side={side ? 'open' : 'closed'} data-files={files ? 'open' : 'closed'}>
        <Sidebar onSearch={openPalette} onSignIn={() => setSignIn(true)} />
        <Panes />
        <RightPanel onOpenFile={openInTab} />
        <Grips />
      </div>

      <StatusStrip />

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
