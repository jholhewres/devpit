import { useState } from 'react'
import type { Project } from '../gen/bindings'
import { currentWorktree } from '../project/surface'
import type { SurfaceId } from '../project/surface'
import { AddProject } from './AddProject'
import {
  CanvasGlyph,
  ChevronGlyph,
  DiagnosticsGlyph,
  NotesGlyph,
  BoardGlyph,
  OverviewGlyph,
  PlusGlyph,
  SettingsGlyph,
  SidebarGlyph,
  WikiGlyph
} from './glyphs'

/**
 * The left column: every project, and everything the open one owns.
 *
 * There is no separate deck and no second list to reconcile. A project is its
 * own workspace, so it opens in place and shows what belongs to it underneath
 * — the row you navigate with is the row that reports state.
 */

const RESOURCES: { id: SurfaceId; label: string; glyph: React.ComponentType }[] = [
  { id: 'board', label: 'Board', glyph: BoardGlyph },
  { id: 'overview', label: 'Overview', glyph: OverviewGlyph },
  { id: 'canvas', label: 'Canvas', glyph: CanvasGlyph },
  { id: 'notes', label: 'Notes', glyph: NotesGlyph },
  { id: 'wiki', label: 'Wiki', glyph: WikiGlyph }
]

/** Two letters carry a name at 22px; three start to look like a word. */
function initials(name: string): string {
  const parts = name.split(/[-_. ]/).filter(Boolean)
  if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase()
  return name.slice(0, 2).toUpperCase()
}

/**
 * How much is uncommitted, or why that is not known.
 *
 * Unread is not zero — the two look identical on screen and one of them is a
 * lie. Never rendered as `+12` either: that is git's own grammar for "ahead by
 * twelve commits", and it is the wrong answer to the wrong question.
 */
function Dirty({ count }: { count: number | null | undefined }): React.JSX.Element | null {
  if (count === undefined) return null
  if (count === null) {
    return (
      <span className="project__dirty" data-unread="true" title="git could not be read">
        · unread
      </span>
    )
  }
  if (count === 0) return null
  return (
    <span className="project__dirty" title={`${count} uncommitted files`}>
      · {count}
    </span>
  )
}

function ProjectRow({
  project,
  open,
  activeSurface,
  paneIds,
  focusedId,
  onOpen,
  onFocusPane,
  onOpenSurface
}: {
  project: Project
  open: boolean
  activeSurface: SurfaceId | null
  paneIds: string[]
  focusedId: string | null
  onOpen: () => void
  onFocusPane: (id: string) => void
  onOpenSurface: (id: SurfaceId) => void
}): React.JSX.Element {
  const worktree = currentWorktree(project)
  // The order the rows animate in, so a freshly opened project reads top-down.
  let step = 0
  const rise = (): React.CSSProperties => ({ '--i': step++ }) as React.CSSProperties

  return (
    <div
      className="project"
      data-active={open}
      data-open={open}
      style={{ '--accent': project.accent } as React.CSSProperties}
    >
      <button type="button" className="project__head" onClick={onOpen} title={project.rootPath}>
        <span className="project__swatch">{initials(project.name)}</span>

        <span className="project__ident">
          <span className="project__name">{project.name}</span>
          <span className="project__branch">
            {/* A project whose repository could not be read says so on the row
                rather than drawing a branch it does not have. */}
            <em>{project.unreadable ? 'not a repository' : (worktree?.branch ?? 'no checkout')}</em>
            <Dirty count={worktree?.dirtyFiles} />
          </span>
        </span>

        {project.worktrees.length > 1 ? (
          <span className="project__tally" title={`${project.worktrees.length} checkouts`}>
            {project.worktrees.length}
          </span>
        ) : null}

        <span className="project__twist">
          <ChevronGlyph />
        </span>
      </button>

      {open ? (
        <div className="project__body">
          <div className="project__subhead" style={rise()}>
            In progress
          </div>
          {paneIds.length === 0 ? (
            <p className="project__empty" style={rise()}>
              No session yet.
            </p>
          ) : (
            paneIds.map((id) => (
              <button
                key={id}
                type="button"
                className="session"
                data-active={id === focusedId}
                data-testid={`session-row-${id}`}
                style={rise()}
                onClick={() => onFocusPane(id)}
              >
                <span className="session__line">
                  <span className="dot" data-live="true" />
                  <span className="session__title">{id.startsWith('leaf_') ? 'shell' : id}</span>
                </span>
              </button>
            ))
          )}

          <div className="project__subhead" style={rise()}>
            Resources
          </div>

          {RESOURCES.map((resource) => {
            const Glyph = resource.glyph
            return (
              <button
                key={resource.id}
                type="button"
                className="resource"
                data-active={activeSurface === resource.id}
                style={rise()}
                onClick={() => onOpenSurface(resource.id)}
              >
                <span className="resource__glyph">
                  <Glyph />
                </span>
                {resource.label}
              </button>
            )
          })}

          <div className="rule" style={rise()} />

          <button
            type="button"
            className="resource"
            data-active={activeSurface === 'diagnostics'}
            style={rise()}
            onClick={() => onOpenSurface('diagnostics')}
          >
            <span className="resource__glyph">
              <DiagnosticsGlyph />
            </span>
            Diagnostics
          </button>
        </div>
      ) : null}
    </div>
  )
}

function groupsOf(projects: Project[]): [string, Project[]][] {
  const groups = new Map<string, Project[]>()
  for (const project of projects) {
    const label = project.group ?? 'Projects'
    const bucket = groups.get(label)
    if (bucket) bucket.push(project)
    else groups.set(label, [project])
  }
  return [...groups]
}

export function Sidebar({
  projects,
  openProjectId,
  activeSurface,
  paneIds,
  focusedId,
  addFailure,
  onOpenProject,
  onFocusPane,
  onOpenSurface,
  onAddProject,
  onCollapse,
  onStartResize
}: {
  projects: Project[]
  openProjectId: string | null
  activeSurface: SurfaceId | null
  paneIds: string[]
  focusedId: string | null
  addFailure: string | null
  onOpenProject: (id: string) => void
  onFocusPane: (id: string) => void
  onOpenSurface: (id: SurfaceId) => void
  onAddProject: (rootPath: string) => void
  onCollapse: () => void
  onStartResize: (event: React.PointerEvent) => void
}): React.JSX.Element {
  const [filter, setFilter] = useState('')
  const [path, setPath] = useState('')
  const [addOpen, setAddOpen] = useState(false)

  const needle = filter.trim().toLowerCase()
  const shown = needle
    ? projects.filter((project) => project.name.toLowerCase().includes(needle))
    : projects

  return (
    <aside className="sidebar">
      {/* Shares the window's top edge with the tab strip, so it drags too —
          otherwise the left third of the titlebar would be dead. */}
      <div className="sidebar__nav" data-tauri-drag-region>
        <span className="sidebar__mark" data-tauri-drag-region>
          devpit
        </span>
        <span className="sidebar__nav-spacer" data-tauri-drag-region />
        <button
          type="button"
          className="icon-button"
          data-active={addOpen}
          title="Add project"
          onClick={() => setAddOpen((was) => !was)}
        >
          <PlusGlyph size={15} />
        </button>
        <button type="button" className="icon-button" title="Hide sidebar" onClick={onCollapse}>
          <SidebarGlyph />
        </button>
      </div>

      {addOpen ? (
        <AddProject
          path={path}
          failure={addFailure}
          onPath={setPath}
          onAdd={onAddProject}
        />
      ) : null}

      <div className="sidebar__find">
        <input
          className="find"
          type="search"
          placeholder="Filter projects"
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
        />
      </div>

      <div className="sidebar__list scroll">
        {projects.length === 0 ? (
          <p className="empty">No projects yet. Add a folder to start.</p>
        ) : shown.length === 0 ? (
          <p className="empty">No project by that name.</p>
        ) : (
          groupsOf(shown).map(([group, members]) => (
            <div key={group}>
              <div className="sidebar__group">{group}</div>
              {members.map((project) => (
                <ProjectRow
                  key={project.id}
                  project={project}
                  open={project.id === openProjectId}
                  activeSurface={activeSurface}
                  paneIds={project.id === openProjectId ? paneIds : []}
                  focusedId={project.id === openProjectId ? focusedId : null}
                  onOpen={() => onOpenProject(project.id)}
                  onFocusPane={onFocusPane}
                  onOpenSurface={onOpenSurface}
                />
              ))}
            </div>
          ))
        )}
      </div>

      <div className="sidebar__foot">
        <span className="sidebar__avatar">JH</span>
        <span className="sidebar__account">local</span>
        <button type="button" className="icon-button" title="Settings">
          <SettingsGlyph />
        </button>
      </div>

      {/* Straddles the seam with the stage, so the grab target covers the
          border instead of sitting next to it. */}
      <div
        className="divider"
        data-edge="sidebar"
        role="separator"
        aria-orientation="vertical"
        onPointerDown={onStartResize}
      />
    </aside>
  )
}
