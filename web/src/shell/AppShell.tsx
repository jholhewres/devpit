import { useCallback, useEffect, useRef, useState } from 'react'
import { DiffView } from '../files/DiffView'
import { FileView } from '../files/FileView'
import { Onboarding } from '../onboarding/Onboarding'
import { SurfaceHost } from '../surfaces/SurfaceHost'
import { commands } from '../gen/bindings'
import type { Project, SessionLayout } from '../gen/bindings'
import { leafIds } from '../session/tree'
import { messageOf, useLoad } from '../project/load'
import { suppressNativeMenu } from './native-menu'
import { currentWorktree } from '../project/surface'
import type { SurfaceId } from '../project/surface'
import { RightPanel } from './RightPanel'
import { Sidebar } from './Sidebar'
import { TabStrip } from './TabStrip'
import { TerminalStage } from './TerminalStage'
import { ResizeEdges, WindowControls } from './WindowChrome'
import './shell.css'

/**
 * The window.
 *
 * Three columns declared once on a grid: projects, stage, panel. A surface is
 * painted over the stage rather than taking a track, which is what keeps the
 * terminal from resizing when something opens beside it.
 *
 * Selection is held per project on purpose: coming back to one two days later
 * has to land you where you left, and that is the whole reason the axis is the
 * project rather than the branch.
 */

const SIDEBAR = { min: 232, max: 420 }
const PANEL = { min: 240, max: 520 }

export function AppShell(): React.JSX.Element {
  const [openProjectId, setOpenProjectId] = useState<string | null>(null)
  const [worktreeByProject, setWorktreeByProject] = useState<Record<string, string>>({})
  const [surfaceByProject, setSurfaceByProject] = useState<Record<string, SurfaceId | null>>({})
  const [addFailure, setAddFailure] = useState<string | null>(null)
  const [onboarded, setOnboarded] = useState(false)
  const [openFile, setOpenFile] = useState<string | null>(null)
  const [openDiff, setOpenDiff] = useState<string | null>(null)
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [panelOpen, setPanelOpen] = useState(true)
  const [layoutByProject, setLayoutByProject] = useState<Record<string, SessionLayout>>({})
  const [tmux, setTmux] = useState<boolean | null>(null)

  // Null means "whatever the token says", so a window that was never dragged
  // still tracks the responsive widths in tokens.css.
  const [sidebarWidth, setSidebarWidth] = useState<number | null>(null)
  const [panelWidth, setPanelWidth] = useState<number | null>(null)
  const [dragging, setDragging] = useState(false)
  const shellRef = useRef<HTMLDivElement>(null)

  // Mounted once for the life of the window: the browser's context menu is a
  // browser's answer to a right-click, and this is not a browser.
  useEffect(suppressNativeMenu, [])

  useEffect(() => {
    void commands.appCapabilities().then((caps) => setTmux(caps.tmux))
  }, [])

  const load = useCallback(() => commands.projectList(), [])
  const { state, reload } = useLoad(load, [])

  // Derived straight from the load state rather than memoized: the array is a
  // new reference on every render, so a `useMemo` over it would recompute each
  // time anyway while claiming not to.
  const projects: Project[] = state.status === 'ready' ? state.data.projects : []
  const project =
    projects.find((candidate) => candidate.id === openProjectId) ?? projects[0] ?? null

  const worktreeId = project ? (worktreeByProject[project.id] ?? null) : null
  const worktree = project
    ? (project.worktrees.find((w) => w.id === worktreeId) ?? currentWorktree(project))
    : null
  const activeSurface = project ? (surfaceByProject[project.id] ?? null) : null
  const layout = project ? (layoutByProject[project.id] ?? null) : null
  const paneIds = layout ? leafIds(layout.tree) : []

  const setLayout = useCallback((next: SessionLayout) => {
    setLayoutByProject((previous) => ({ ...previous, [next.projectId]: next }))
  }, [])

  const setSurface = (surface: SurfaceId | null): void => {
    if (!project) return
    setSurfaceByProject((previous) => ({ ...previous, [project.id]: surface }))
  }

  /**
   * Opening a project is a write.
   *
   * It records what you touched last, and that is what orders the list next
   * time — so the command answers with the new list rather than leaving this
   * side to guess how the order changed.
   */
  const openProject = (id: string): void => {
    setOpenProjectId(id)
    void commands.projectOpen(id).catch(() => undefined)
    void commands.sessionLayout(id).then((answer) => {
      if (answer.status === 'ok') setLayout(answer.data)
    })
  }

  const split = (): void => {
    if (!project || !layout) return
    const leaf = layout.focusedId
    void commands
      .sessionSplit(project.id, leaf, 'horizontal', worktree?.id ?? null)
      .then((answer) => {
        if (answer.status === 'ok') setLayout(answer.data)
      })
  }

  const focusPane = (leafId: string): void => {
    if (!project || leafId === layout?.focusedId) return
    void commands.sessionFocus(project.id, leafId).then((answer) => {
      if (answer.status === 'ok') setLayout(answer.data)
    })
  }

  const addProject = async (rootPath: string): Promise<void> => {
    setAddFailure(null)
    try {
      const answer = await commands.projectAdd(rootPath)
      if (answer.status === 'error') {
        setAddFailure(answer.error.message)
        return
      }
      setOpenProjectId(answer.data.id)
      reload()
    } catch (thrown) {
      setAddFailure(messageOf(thrown))
    }
  }

  /**
   * Column drag.
   *
   * Pointer capture rather than window listeners: the events keep arriving
   * while the pointer is over the terminal or outside the window, and the
   * capture is released for us if the drag is interrupted.
   */
  const startResize = useCallback(
    (edge: 'sidebar' | 'panel') => (event: React.PointerEvent) => {
      event.preventDefault()
      const handle = event.currentTarget as HTMLElement
      handle.setPointerCapture(event.pointerId)
      setDragging(true)

      const move = (moved: PointerEvent): void => {
        const box = shellRef.current?.getBoundingClientRect()
        if (!box) return
        if (edge === 'sidebar') {
          const next = moved.clientX - box.left
          setSidebarWidth(Math.min(SIDEBAR.max, Math.max(SIDEBAR.min, next)))
        } else {
          const next = box.right - moved.clientX
          setPanelWidth(Math.min(PANEL.max, Math.max(PANEL.min, next)))
        }
      }

      const stop = (): void => {
        setDragging(false)
        handle.removeEventListener('pointermove', move)
        handle.removeEventListener('pointerup', stop)
        handle.removeEventListener('pointercancel', stop)
      }

      handle.addEventListener('pointermove', move)
      handle.addEventListener('pointerup', stop)
      handle.addEventListener('pointercancel', stop)
    },
    []
  )

  // The first run floats over the real window rather than replacing it, so the
  // first thing seen is the app. `onboarded` covers the gap between the last
  // step and the list coming back with the new project — without it the flow
  // would flash back to step one for a frame.
  const firstRun = state.status === 'ready' && projects.length === 0 && !onboarded

  return (
    <>
      {/* The frame the window manager used to provide, outside the shell so
          neither is clipped by it. The controls are pinned to the window and
          not placed in a column: the right panel is what touches the right
          edge, and a column can be closed, dragged narrow or left empty. */}
      <ResizeEdges />
      <WindowControls />

    <div
      ref={shellRef}
      className="shell"
      data-sidebar={sidebarOpen ? 'open' : 'closed'}
      data-panel={panelOpen ? 'open' : 'closed'}
      data-dragging={dragging}
      style={
        {
          // The project colour is set once at the root and every column reads
          // it from here. It is persistent and mandatory: it is the thing that
          // keeps one client's context from being read as another's.
          '--accent': project?.accent ?? 'var(--sev-neutral)',
          ...(sidebarWidth === null ? {} : { '--sidebar-w': `${sidebarWidth}px` }),
          ...(panelWidth === null ? {} : { '--right-w': `${panelWidth}px` })
        } as React.CSSProperties
      }
    >
      {sidebarOpen ? (
        <Sidebar
          projects={projects}
          openProjectId={project?.id ?? null}
          activeSurface={activeSurface}
          paneIds={paneIds}
          focusedId={layout?.focusedId ?? null}
          addFailure={addFailure}
          onOpenProject={openProject}
          onFocusPane={focusPane}
          onOpenSurface={(surface) => setSurface(activeSurface === surface ? null : surface)}
          onAddProject={(path) => void addProject(path)}
          onCollapse={() => setSidebarOpen(false)}
          onStartResize={startResize('sidebar')}
        />
      ) : (
        <div className="sidebar-stub" />
      )}

      <div className="stage">
        <TabStrip
          worktree={worktree}
          paneCount={paneIds.length}
          focusedId={layout?.focusedId ?? null}
          sidebarOpen={sidebarOpen}
          panelOpen={panelOpen}
          canSplit={Boolean(layout && tmux)}
          onToggleSidebar={() => setSidebarOpen((was) => !was)}
          onTogglePanel={() => setPanelOpen((was) => !was)}
          onSplit={split}
        />

        {/* Three states and no fourth: reading, a failure with its sentence,
            or a project. An empty list is a fourth thing and gets its own
            words rather than an empty stage. */}
        {state.status === 'loading' ? (
          <div className="terminal-stage">
            <div className="line" data-kind="dim">
              Reading projects…
            </div>
          </div>
        ) : state.status === 'failed' ? (
          <div className="terminal-stage">
            <div className="line" data-kind="error">
              {state.message}
            </div>
          </div>
        ) : project === null ? (
          <div className="terminal-stage">
            <div className="line" data-kind="dim">
              Select a project.
            </div>
          </div>
        ) : (
          <TerminalStage
            project={project}
            worktree={worktree}
            layout={layout}
            onLayout={setLayout}
            tmux={tmux}
          />
        )}

        {project && openDiff !== null ? (
          <div className="surface-host">
            <DiffView
              projectId={project.id}
              worktreeId={worktreeId}
              path={openDiff}
              onClose={() => setOpenDiff(null)}
            />
          </div>
        ) : null}

        {project && openFile !== null ? (
          // Over the stage like any other surface, and it hands the terminal
          // back on close: the pane underneath keeps its process either way.
          <div className="surface-host">
            <FileView
              projectId={project.id}
              worktreeId={worktreeId}
              path={openFile}
              onClose={() => setOpenFile(null)}
            />
          </div>
        ) : null}

        {project && activeSurface ? (
          <SurfaceHost
            // Remounting on change is what replays the entrance: without it,
            // moving between two surfaces swaps the body with no transition.
            key={`${project.id}:${activeSurface}`}
            surface={activeSurface}
            project={project}
            worktree={worktree}
            worktreeId={worktreeId}
            onClose={() => setSurface(null)}
          />
        ) : null}
      </div>

      {panelOpen && project ? (
        <RightPanel
          project={project}
          worktreeId={worktreeId}
          onSelectWorktree={(id) =>
            setWorktreeByProject((previous) => ({ ...previous, [project.id]: id }))
          }
          onOpenFile={(path) => {
            setOpenDiff(null)
            setOpenFile(path)
          }}
          onOpenDiff={(path) => {
            setOpenFile(null)
            setOpenDiff(path)
          }}
          onStartResize={startResize('panel')}
        />
      ) : (
        <div className="panel-stub" />
      )}

      {firstRun ? (
        <Onboarding onProjectAdded={reload} onDone={() => setOnboarded(true)} />
      ) : null}
    </div>
    </>
  )
}
