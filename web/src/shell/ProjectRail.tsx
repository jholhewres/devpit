import { useEffect, useMemo, useRef, useState } from 'react'

import type { Project } from '../gen/bindings'
import { Close, Folder, Pencil, Plus, Trash } from './GitIcons'
import { ask, commands } from './live'
import { ProjectDialog } from './ProjectDialog'
import { ProjectMark } from './ProjectMark'
import { RailGroup } from './RailGroup'
import { RailMenu, type RailItem } from './RailMenu'
import { activeOnly, moved, ordered, saveActiveOnly, savedActiveOnly, saveGroups, savedGroups, saveOrder, savedOrder, saveShut, savedShut, sections, shown } from './rail'
import { remembered } from './tabs'
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
 * somewhere else, and the order is the person's. Groups fold, and are renamed
 * or dissolved from their own heading.
 */

type Menu = { x: number; y: number; items: readonly RailItem[] }

/** What is being dragged: a project, or a whole group by its heading. */
type Drag = { kind: 'project'; id: string } | { kind: 'group'; name: string }

export function ProjectRail({ onAddProject, onRemove }: { onAddProject: () => void; onRemove: (id: string) => void }): React.JSX.Element {
  const { projects, project, setProject, open, reloadProjects } = useShell()
  const [order, setOrder] = useState<readonly string[]>(savedOrder)
  const [shut, setShut] = useState<ReadonlySet<string>>(savedShut)
  const [dragging, setDragging] = useState<Drag | null>(null)
  const [over, setOver] = useState<string | null>(null)
  const [groupOrder, setGroupOrder] = useState<readonly string[]>(savedGroups)
  const [onlyActive, setOnlyActive] = useState(savedActiveOnly)
  const [editing, setEditing] = useState<Project | null>(null)
  const [renaming, setRenaming] = useState<string | null>(null)
  const [menu, setMenu] = useState<Menu | null>(null)
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

  const tabsOf = (id: string): number => (id === project?.id ? open.length : remembered(id).open.length)

  const all = useMemo(() => sections(list, groupOrder), [list, groupOrder])
  /* Active is tabs open; the project in front always stays, or the filter
     would hide where you are. */
  const cut = onlyActive ? activeOnly(all, (one) => one.id === project?.id || tabsOf(one.id) > 0) : all

  const done = (): void => {
    setDragging(null)
    setOver(null)
  }

  /* A project into another group keeps everything else about it; only the
     group changes, the same edit the dialog makes. */
  const regroup = (one: Project, group: string | null): Promise<unknown> =>
    (one.group ?? null) === group
      ? Promise.resolve()
      : ask(() => commands.projectEdit(one.id, one.name, group, one.icon, one.color)).then(() => reloadProjects())

  /** A project dropped on another: placed where it is, in its group. */
  const dropOnProject = (target: Project): void => {
    if (dragging?.kind !== 'project' || dragging.id === target.id) return done()
    const moving = list.find((one) => one.id === dragging.id)
    const next = moved(list.map((one) => one.id), dragging.id, target.id)
    setOrder(next)
    saveOrder(next)
    if (moving) void regroup(moving, target.group ?? null)
    done()
  }

  /** On a heading: a project joins the group; a group takes that place. */
  const dropOnGroup = (target: string): void => {
    if (dragging?.kind === 'project') {
      const moving = list.find((one) => one.id === dragging.id)
      if (moving) void regroup(moving, target)
    } else if (dragging?.kind === 'group' && dragging.name !== target) {
      const names = all.flatMap((one) => (one.group ? [one.group] : []))
      const next = moved(names, dragging.name, target)
      setGroupOrder(next)
      saveGroups(next)
    }
    done()
  }

  /* Only what the target can take shows it would: a group over a project is
     not a move anywhere. */
  const takes = (target: 'project' | 'group'): boolean =>
    dragging !== null && (target === 'group' || dragging.kind === 'project')

  const fold = (group: string): void => {
    const next = new Set(shut)
    if (next.has(group)) next.delete(group)
    else next.add(group)
    setShut(next)
    saveShut(next)
  }

  const renameGroup = (from: string, to: string): void => {
    void ask(() => commands.projectGroupRename(from, to)).then(() => {
      reloadProjects()
      /* The fold follows the group to its new name. */
      if (shut.has(from)) {
        const next = new Set(shut)
        next.delete(from)
        if (to.trim()) next.add(to.trim())
        setShut(next)
        saveShut(next)
      }
    })
  }

  const projectMenu = (one: Project): readonly RailItem[] => [
    { label: 'Edit project…', glyph: <Pencil />, act: () => setEditing(one) },
    { label: 'Reveal folder', glyph: <Folder />, act: () => void ask(() => commands.pathReveal(one.rootPath)) },
    'rule',
    { label: 'Remove…', glyph: <Trash />, act: () => onRemove(one.id), bad: true },
  ]

  const groupMenu = (group: string): readonly RailItem[] => [
    { label: 'Rename group…', glyph: <Pencil />, act: () => setRenaming(group) },
    { label: 'Ungroup', glyph: <Close />, act: () => renameGroup(group, '') },
  ]

  return (
    <nav className="rail" aria-label="Projects">
      <div className="rail__panel">
        <button
          className="rail__filter"
          aria-pressed={onlyActive}
          title={onlyActive ? 'Show every project' : 'Show only projects with tabs open'}
          onClick={() => {
            setOnlyActive(!onlyActive)
            saveActiveOnly(!onlyActive)
          }}
        >
          <span className="rail__fdot" />
          <span className="rail__ftext">{onlyActive ? 'Active projects' : 'All projects'}</span>
        </button>
        <div className="rail__list">
          {cut.map((section) => {
            const folded = section.group !== null && shut.has(section.group)
            return (
              <div className="rail__sect" key={section.group ?? ''} data-folded={folded ? 'true' : undefined}>
                {section.group && (
                  <RailGroup
                    name={section.group}
                    folded={folded}
                    over={over === `group:${section.group}`}
                    renaming={renaming === section.group}
                    onToggle={() => fold(section.group!)}
                    onMenu={(at) => setMenu({ ...at, items: groupMenu(section.group!) })}
                    onRename={(to) => {
                      if (renaming !== section.group) return
                      setRenaming(null)
                      if (to !== null && to.trim() !== section.group) renameGroup(section.group!, to)
                    }}
                    drag={{
                      onDragStart: (event) => {
                        event.dataTransfer.effectAllowed = 'move'
                        setDragging({ kind: 'group', name: section.group! })
                      },
                      onDragEnd: done,
                      onDragOver: (event) => {
                        if (!takes('group')) return
                        event.preventDefault()
                        setOver(`group:${section.group}`)
                      },
                      onDragLeave: () => setOver(null),
                      onDrop: () => dropOnGroup(section.group!),
                    }}
                  />
                )}
                {shown(section, folded).map((one) => {
                  const tabs = tabsOf(one.id)
                  const branch = (one.worktrees.find((tree) => tree.current) ?? one.worktrees[0])?.branch
                  return (
                    <button
                      key={one.id}
                      className="rail__i"
                      aria-current={one.id === project?.id ? 'true' : undefined}
                      data-active={tabs > 0 ? 'true' : undefined}
                      data-unreadable={one.unreadable ? 'true' : undefined}
                      data-dragging={dragging?.kind === 'project' && dragging.id === one.id ? 'true' : undefined}
                      data-over={over === one.id ? 'true' : undefined}
                      title={one.unreadable ?? undefined}
                      draggable
                      onDragStart={(event) => {
                        event.dataTransfer.effectAllowed = 'move'
                        setDragging({ kind: 'project', id: one.id })
                      }}
                      onDragEnd={done}
                      onDragOver={(event) => {
                        if (!takes('project')) return
                        event.preventDefault()
                        setOver(one.id)
                      }}
                      onDragLeave={() => setOver(null)}
                      onDrop={() => dropOnProject(one)}
                      onClick={() => one.id !== project?.id && setProject(one.id)}
                      onContextMenu={(event) => {
                        event.preventDefault()
                        event.stopPropagation()
                        setMenu({ x: event.clientX, y: event.clientY, items: projectMenu(one) })
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
            )
          })}
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

      {menu && <RailMenu at={menu} items={menu.items} onClose={() => setMenu(null)} />}
      {editing && <ProjectDialog project={editing} onClose={() => setEditing(null)} />}
    </nav>
  )
}
