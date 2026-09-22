import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

import type { Project } from '../gen/bindings'
import { Folder, Pencil, Plus, Trash } from './GitIcons'
import { ask, commands } from './live'
import { ProjectDialog } from './ProjectDialog'
import { ProjectMark } from './ProjectMark'
import { moved, ordered, saveOrder, savedOrder, sections } from './rail'
import { remembered } from './tabs'
import { abandoned } from './typing'
import { useShell } from './useShell'

/*
 * Every project, one icon each, down the left edge.
 *
 * The picker in the top bar hid the others behind a click. The rail keeps them
 * all in sight the way a chat app keeps its servers — narrow enough to cost
 * nothing, and widening over the content, never beside it, to show names
 * while the pointer is on it. The terminal under it never changes size.
 *
 * A project with tabs open is *active*, and says so with a dot: that is where
 * work was left. Right-click edits its name, group and mark; dragging puts it
 * somewhere else, and the order is the person's.
 */

export function ProjectRail({ onAddProject, onRemove }: { onAddProject: () => void; onRemove: (id: string) => void }): React.JSX.Element {
  const { projects, project, setProject, open } = useShell()
  const [order, setOrder] = useState<readonly string[]>(savedOrder)
  const [dragging, setDragging] = useState<string | null>(null)
  const [editing, setEditing] = useState<Project | null>(null)
  const [menu, setMenu] = useState<{ x: number; y: number; project: Project } | null>(null)
  const list = useMemo(() => ordered(projects, order), [projects, order])

  /* A project that was not here a moment ago was just added: offer its mark
     while it is the thing being looked at. Not on the first list, which is
     every project there already was. */
  const known = useRef<ReadonlySet<string> | null>(null)
  useEffect(() => {
    const ids = new Set(projects.map((one) => one.id))
    const was = known.current
    known.current = ids
    if (!was || was.size === 0) return
    const added = projects.find((one) => !was.has(one.id))
    if (added) setEditing(added)
  }, [projects])

  const active = (id: string): boolean => (id === project?.id ? open.length > 0 : remembered(id).open.length > 0)

  const drop = (to: string): void => {
    if (!dragging) return
    const next = moved(list.map((one) => one.id), dragging, to)
    setOrder(next)
    saveOrder(next)
    setDragging(null)
  }

  return (
    <nav className="rail" aria-label="Projects">
      <div className="rail__panel">
        <div className="rail__list">
          {sections(list).map((section) => (
            <div className="rail__sect" key={section.group ?? ''}>
              {section.group && <div className="rail__group" title={section.group}><span>{section.group}</span></div>}
              {section.projects.map((one) => {
                const tabs = one.id === project?.id ? open.length : remembered(one.id).open.length
                const branch = (one.worktrees.find((tree) => tree.current) ?? one.worktrees[0])?.branch
                return (
                  <button
                    key={one.id}
                    className="rail__i"
                    aria-current={one.id === project?.id ? 'true' : undefined}
                    data-active={active(one.id) ? 'true' : undefined}
                    data-unreadable={one.unreadable ? 'true' : undefined}
                    data-dragging={dragging === one.id ? 'true' : undefined}
                    title={one.unreadable ?? undefined}
                    draggable
                    onDragStart={() => setDragging(one.id)}
                    onDragEnd={() => setDragging(null)}
                    onDragOver={(event) => {
                      if (dragging) event.preventDefault()
                    }}
                    onDrop={() => drop(one.id)}
                    onClick={() => one.id !== project?.id && setProject(one.id)}
                    onContextMenu={(event) => {
                      event.preventDefault()
                      event.stopPropagation()
                      setMenu({ x: event.clientX, y: event.clientY, project: one })
                    }}
                  >
                    <span className="rail__pill" />
                    <span className="rail__ico">
                      <ProjectMark project={one} />
                      <span className="rail__dot" />
                    </span>
                    <span className="rail__text">
                      <span className="rail__n">{one.name}</span>
                      <span className="rail__s">
                        {branch && <span className="rail__b">{branch}</span>}
                        {tabs > 0 && <span className="rail__t">{tabs} tab{tabs === 1 ? '' : 's'}</span>}
                      </span>
                    </span>
                  </button>
                )
              })}
            </div>
          ))}
        </div>
        <button className="rail__i rail__add" onClick={onAddProject} title="Add a project">
          <span className="rail__pill" />
          <span className="rail__ico">
            <span className="pmark pmark--add">
              <Plus size={16} />
            </span>
          </span>
          <span className="rail__text">
            <span className="rail__n">Add project</span>
          </span>
        </button>
      </div>

      {menu && (
        <RailMenu
          at={menu}
          onClose={() => setMenu(null)}
          onEdit={() => setEditing(menu.project)}
          onReveal={() => void ask(() => commands.pathReveal(menu.project.rootPath))}
          onRemove={() => onRemove(menu.project.id)}
        />
      )}
      {editing && <ProjectDialog project={editing} onClose={() => setEditing(null)} />}
    </nav>
  )
}

function RailMenu({
  at,
  onClose,
  onEdit,
  onReveal,
  onRemove,
}: {
  at: { x: number; y: number }
  onClose: () => void
  onEdit: () => void
  onReveal: () => void
  onRemove: () => void
}): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  useLayoutEffect(() => {
    const el = box.current
    if (!el) return
    const size = el.getBoundingClientRect()
    el.style.top = `${Math.min(at.y, window.innerHeight - size.height - 8)}px`
  }, [at])
  useEffect(() => {
    const outside = (event: MouseEvent): void => {
      if (!box.current?.contains(event.target as Node)) onClose()
    }
    const key = (event: KeyboardEvent): void => {
      if (abandoned(event)) onClose()
    }
    document.addEventListener('mousedown', outside, true)
    document.addEventListener('keydown', key)
    return () => {
      document.removeEventListener('mousedown', outside, true)
      document.removeEventListener('keydown', key)
    }
  }, [onClose])
  const item = (label: string, glyph: React.ReactNode, act: () => void, bad = false): React.JSX.Element => (
    <button
      className={bad ? 'ctx__i ctx__i--bad' : 'ctx__i'}
      role="menuitem"
      onClick={() => {
        onClose()
        act()
      }}
    >
      <span className="ctx__g">{glyph}</span>
      {label}
    </button>
  )
  return createPortal(
    <div className="ctx" ref={box} role="menu" style={{ left: at.x, top: at.y }}>
      {item('Edit project…', <Pencil />, onEdit)}
      {item('Reveal folder', <Folder />, onReveal)}
      <div className="ctx__rule" />
      {item('Remove…', <Trash />, onRemove, true)}
    </div>,
    document.body,
  )
}
